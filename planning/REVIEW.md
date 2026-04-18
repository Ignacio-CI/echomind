# EchoMind — Project Review

_Date: 2026-04-18_

This review compares the current state of the repo against `planning/PLAN.md`, documents the blocker visible in the UI ("Failed to start recording: Config parsing error…"), and flags architectural issues worth fixing before moving on.

---

## 1. Current blocker — why "Start Recording" fails

**UI error.** `Failed to start recording: Config parsing error: invalid type: null, expected usize. This model might require a newer version of candle-transformers.`

**Where it comes from.** `src-tauri/src/ai/gemma.rs:101-102`. The message is a custom-formatted error wrapped around `serde_json::from_value::<Config>(…)`. The `Config` is `candle_transformers::models::gemma::Config` (line 3).

**Why the parse fails — verified against the real config.** The repo `google/gemma-4-E2B-it` **does exist** (public, not gated, last modified 2026-04-10, `Gemma4ForConditionalGeneration`, `model_type: gemma4`, `pipeline_tag: any-to-any`). Its `config.json` is **nested** — top-level keys include `text_config`, `vision_config`, `audio_config` plus cross-modal token ids. The null fields that break serde are **inside `text_config`**:

```
text_config.expert_intermediate_size       = null
text_config.num_experts                    = null
text_config.num_global_key_value_heads     = null
text_config.top_k_experts                  = null
text_config.use_bidirectional_attention    = null
```

The shim in `gemma.rs:64-72` only walks the **top level**, so the nulls inside `text_config` are handed straight to serde. That's exactly the "invalid type: null, expected usize" surface. The six field patches at lines 74-99 (`attention_bias`, `head_dim`, `num_key_value_heads`, `max_position_embeddings`, `rope_theta`, `rms_norm_eps`) are also applied to the top-level object and never reach `text_config`, where they'd need to live for a Gemma 4 config.

**Root cause (the real one).** Patching `text_config` won't fix this either, because the **module is wrong**. `candle_transformers::models::gemma::Config` is the **Gemma 1** config (flat schema, different field set). Gemma 4 has its own forward pass (MoE blocks, per-layer input sizes, sliding-window attention layer types, global head dim, final-logit softcapping, multimodal token ids). You cannot coerce it into Gemma 1 with JSON edits.

**candle-transformers support matrix** (checked against the crate on crates.io and the GitHub tags, 2026-04-18):

| candle-transformers version | `gemma` | `gemma2` | `gemma3` | `gemma4` |
| --- | :---: | :---: | :---: | :---: |
| 0.9.x (pinned in `Cargo.toml`) | ✅ | ✅ | ✅ | ❌ |
| 0.10.0 / 0.10.1 / 0.10.2 (latest published) | ✅ | ✅ | ✅ | ❌ |
| `main` (unreleased) | ✅ | ✅ | ✅ | ✅ |

So there is **no released candle-transformers with a `gemma4` module**. Options:

1. **Switch to a Gemma generation candle actually ships.** For a text-only meeting-summarizer, `google/gemma-3-1b-it` (≈1B params, text-only, instruction-tuned) or `google/gemma-2-2b-it` fits the job. Use `models::gemma3` / `models::gemma2` respectively. This is the pragmatic path and matches the spirit of PLAN step 7.
2. **Track candle `main` as a git dependency** to get `gemma4`. Moving target; also pulls in an unreleased API surface the rest of the code (whisper) depends on.
3. **Wait for the next tagged release** that includes `gemma4`.

One more thing about option 1: `google/gemma-4-E2B-it` is a **multimodal** model (`any-to-any`, text + image + audio). EchoMind's pipeline feeds it **text transcripts only**. Running a multimodal Gemma 4 for a text-only summarization task is over-specced and costs RAM + download size for vision/audio encoders we never use. Even if `gemma4` support lands in candle, a text-only Gemma 2/3 is a better fit for this project.

**Recommendation for step 7 of the PLAN.** Pin `google/gemma-3-1b-it` (or `2b-it` if RAM allows), record the choice in `README.md` (PLAN appendix), and use `candle_transformers::models::gemma3`. Delete the JSON-shim code in `gemma.rs`; a correctly-matched `Config` parses cleanly.

**Side effect on UX.** `commands::audio_start` (`commands.rs:59-132`) chains `audio::start → whisper::load → gemma::load` in one awaitable. When `gemma::load` returns `Err`, the function returns before any cleanup — but `audio::start` already spawned the capture thread and set `CHUNK_TX`. Pressing "Start Recording" again then hits the "Already recording" guard. You won't recover without calling `audio_stop` first (which the UI only sends when `isRecording = true`, which it isn't, because the start failed). **This is why the toast appears but the app is now in a half-started state.**

---

## 2. PLAN.md status — step by step

Legend: ✅ done · 🟡 partial / has issues · ❌ not started

### ✅ Step 1 — Project scaffold
Tauri 2 + SvelteKit + adapter-static is in place. `tauri.conf.json` has product name, identifier, and Vite dev URL (1420). **Gap:** package-manager choice isn't recorded at the top of `README.md`; the README is still the default template (`README.md:1`).

### ✅ Step 2 — UI foundation
Tailwind v4 via `@tailwindcss/vite`, shadcn-svelte components (`button`, `card`, `tabs`, `sonner`, `dialog`, `tooltip`, …) installed. Shell in `src/routes/+layout.svelte` + `+page.svelte` with top bar, main panel, footer. `ai-elements` (message, reasoning) present. ThemeWatcher via `mode-watcher`.

### ✅ Step 3 — i18n
`src/lib/i18n/{en,es,index.svelte.ts}` exist. `t(key)` is called across the page. Language toggle button is in the top bar. **Not verified here:** OS-locale default and `localStorage` persistence — worth a smoke test.

### ✅ Step 4 — Rust backend skeleton
Modules `audio/`, `ai/`, `storage/`, `commands.rs`, `events.rs`, `error.rs` exist. `AppError` via `thiserror`. Event constants defined. `ping` command round-trips (footer shows `Bridge: pong`). `src/lib/bridge.ts` wraps invoke/listen.

### 🟡 Step 5 — Audio capture
Mic capture works (device enumeration, default device, PCM → ringbuf, 50 ms RMS events). `audio_start` / `audio_stop` / `audio_list_devices` exposed. **Issues:**
- **Downsampling is naive decimation** (`audio/mod.rs:83-101`): `sample_rate / 16000` rounded to an integer, then picks every Nth sample. This aliases badly at 44.1 kHz → 16 kHz (ratio 2.75 rounds to 3; drift + aliasing). Whisper is sensitive to this. Use a proper resampler (`rubato`) or the anti-aliased path.
- **RMS is computed over a 50 ms buffer of size `frame_size`** — but `cons.pop_slice` may return `n < frame_size`. Current code uses `level_buf[..n]`, which is fine, but the accumulator logic silently drops samples when the ringbuf overflows (`try_push` failures are ignored at line 67).
- **macOS loopback** (BlackHole / ScreenCaptureKit) isn't addressed. PLAN says this is OK to defer, but nothing in `README.md` tells the user how to route system audio.
- **Device selector UI** isn't wired — `audio_list_devices` exists but nothing calls it.

### 🟡 Step 6 — Whisper (Candle)
Loads `openai/whisper-small` (`ai/whisper.rs:13-14`) with a hand-rolled mel-filterbank (lines 32-64) and a manual greedy decoder (lines 162-191). **Issues:**
- **Greedy decode over 224 steps with no KV cache management, no timestamp tokens, no language tokens, no no-speech filtering.** Output quality and latency will be poor.
- **Single global `Mutex<WhisperModel>`** on Metal: when a chunk takes longer than the 5 s cadence, chunks back up in the `mpsc::sync_channel` (capacity 4), and new captures eventually block.
- **No language detection** — the PLAN wants detected language to flow into the Gemma prompt (step 9).
- **Never called because Gemma fails first** (see §1). Whisper's own load path is otherwise fine, but has not been exercised end-to-end.

### ❌ Step 7 — Local Gemma 4 inference
**Broken** — see §1. Also:
- `candle_transformers::models::gemma::Model` is used (Gemma 1). Wrong architecture for any 2024+ Gemma.
- Quantization isn't set up; `DType::F32` on `model.safetensors` means the full-precision weights are mmap'd into RAM. PLAN says 4-bit (GGUF or Candle quantized tensors).
- Streaming decode uses `tokenizer.id_to_token` per step and patches `<0x0A>` → `\n` manually (`gemma.rs:172-175`). Proper handling needs a streaming detokenizer that flushes on byte-fallback + whitespace boundaries, otherwise UTF-8 fragments render as mojibake. A naive replace of ASCII space with ASCII space (line 173) is a no-op — looks like a placeholder.
- No chat template. The prompt is plain text concatenation (`prompts.rs:5-8`), but Gemma instruction-tuned models expect `<start_of_turn>user … <end_of_turn><start_of_turn>model`. Without the template, output quality drops sharply.
- EOS check is hard-coded to token id `1` (`gemma.rs:177`). The actual EOS token id for Gemma should be read from the tokenizer / config.

### 🟡 Step 8 — Thinking Mode UI
`ThinkingPanel.svelte` exists and the layout uses it. Not verified for smoothness end-to-end because Gemma never streams.

### 🟡 Step 9 — Meeting pipeline
Implemented in `commands::audio_start` + `ai::meeting`. **Issues:**
- **Locale hard-coded to `"en"`** in the auto-summarizer (`commands.rs:113`). PLAN wants detected spoken language to drive the summary language.
- **Auto-summaries are not persisted.** `meeting_summarize` saves (`commands.rs:158`), but the background task in `audio_start` (`commands.rs:94-129`) only emits the event — it never calls `storage::summary_save`.
- **Trigger logic is loose.** `last_summary_time` resets on every successful run, including when the transcript hasn't grown since the previous summary, producing back-to-back summaries on silence.
- **Rolling cap is `transcript.len() > 100` entries** (`meeting.rs:32-34`). That's not the "last 10 minutes or N tokens" cap from PLAN — 100 5-second windows is only ~8 min, and token count can balloon well past Gemma's context depending on length of each line.

### 🟡 Step 10 — Persistence (SQLite)
`rusqlite` chosen, schema matches PLAN (`storage/schema.sql`), migrations run on startup. CRUD commands exist. **Issues:**
- `ended_at` is never written anywhere — `audio_stop` doesn't mark the meeting as ended. The schema's column is just dead weight today.
- `action_items_json` column is in the schema but `summary_save` doesn't populate it (`storage/mod.rs:104-111`).
- `TranscriptEntry.t_start` / `t_end` are always `0.0` (`meeting.rs:43-46`), so the `ORDER BY t_start ASC` on read is effectively insertion order via id — fine today but misleading.
- No settings UI is wired to `settings_get/set`.
- `meeting_save_summary` exists but isn't listed in `lib.rs:invoke_handler!`. It's called from Rust only — OK, but inconsistent with the PLAN's "CRUD commands."

_User note: "SQL db related and forward steps haven't yet been implemented."_ Storage code is written, but end-to-end (write on record, read on history) is not exercised because recording can't start.

### ❌ Step 11 — Overlay mode
Not started. `tauri.conf.json` has a single window. No `/overlay` route. No global shortcut.

### ❌ Step 12 — Polish & release
- No `tracing` / `tracing-appender` logging — `println!` in `commands.rs:60,64,66,69,74`.
- No Sonner error-toast plumbing beyond the inline `toast.error` in `+page.svelte:64`.
- README is still the default Tauri template.
- No signing docs, no icon set beyond defaults.

### ❌ Step 13 — Verification & testing
No `cargo test` targets. No svelte-check / lint scripts beyond `pnpm check`. No smoke-test script.

---

## 3. Other observations (not called out in PLAN but worth fixing)

1. **`audio_start` does two things at once.** It starts audio AND loads two multi-hundred-MB models on the same call. First-run latency is brutal, and a model-load failure aborts everything after the audio thread is already alive (see §1 "half-started state"). Separate: `models_prepare` (explicit, with progress events) vs `audio_start` (cheap, just streams).

2. **`audio_start` leaks on partial failure.** If `whisper::load` or `gemma::load` fails, `audio::RECORDER` and `CHUNK_TX` are still set. Callers see the error but the app is stuck until `audio_stop` is called — which the UI refuses to do because `isRecording` is `false`.

3. **Unsafe `Send`/`Sync` for `WhisperModel`** (`whisper.rs:25-26`) is asserted without a comment justifying it. Metal tensors are not `Send` in general. This will surface as runtime crashes or UB under load. Prefer running the model on a single owned thread with a command channel.

4. **`Tokenizer::from_file`** is fine, but the `tokenizers` crate is compiled with `default-features = false` + `onig` (`Cargo.toml:32`). If upstream starts requiring the default pre-tokenizer regex engine, things break. Worth pinning deliberately.

5. **`Lucide` duplicated.** `lucide-svelte ^1.0.1` and `@lucide/svelte ^1.8.0` are both installed (`package.json:21,30`). Pick one.

6. **CSP is `null`** in `tauri.conf.json:21`. Fine for dev, needs tightening before release.

7. **Commit hygiene.** No `.gitignore` observed for `src-tauri/target` — worth verifying (PLAN step 1).

---

## 4. Recommended next steps (ordered)

1. **Unblock recording** by switching Gemma 4 → Gemma 3. Pin `google/gemma-3-1b-it` (or `2b-it`), use `candle_transformers::models::gemma3`, delete the JSON-patching shim. Record the decision in `README.md` per PLAN appendix. Gemma 4 via candle isn't possible on a released crate today (see §1); and it's multimodal, which EchoMind doesn't need.
2. **Decouple model load from `audio_start`.** New commands: `models_status`, `models_prepare` (emits progress). `audio_start` should only fail for audio reasons.
3. **Make `audio_start` transactional** — on any error after the stream is built, call the same cleanup path as `audio_stop`.
4. **Fix the resampler** in `audio/mod.rs` (use `rubato`) before tuning anything in Whisper.
5. **Persist auto-summaries** and write `ended_at` on `audio_stop`; that finishes step 10.
6. **Detect language from Whisper** and thread it through to the Gemma prompt; that finishes step 9.
7. Then move on to step 11 (overlay) and step 12 (polish).

---

## 5. Decisions that are still open

From PLAN's appendix, none of these have been recorded in `README.md`:

- Frontend package manager (implicitly pnpm, per `tauri.conf.json:7,9` and `pnpm-lock.yaml`).
- Gemma variant + quantization.
- SQLite crate (implicitly `rusqlite`).
- macOS loopback strategy (unaddressed).

Pin them in `README.md` as soon as each is made — that's the lightest-weight ADR and the PLAN already asks for it.
