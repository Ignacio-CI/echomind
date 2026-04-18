# EchoMind — Step-by-Step Build Plan

## 0. Preamble

**Project.** EchoMind is a private, open-source desktop app (Tauri + Rust) that captures system + mic audio, transcribes it locally, and uses a local Gemma 4 model (via Candle) to act as a real-time meeting analyst. Data never leaves the machine. The UI is Svelte 5 (Runes) with shadcn-svelte + shadcn-ai-elements, and the app supports English and Spanish.

**Ground rules (from `CLAUDE.md`).**
- Build **incrementally**: small, simple steps. Validate each increment before moving on.
- Use the **latest stable APIs** (Tauri 2, Svelte 5 Runes, Candle current release).
- **Do not overengineer.** No defensive programming. No speculative abstractions.
- **Prove before fixing.** Identify the root cause with evidence, then fix — no guessing, no workarounds.
- Frontend package manager is decided in step 1 and respected thereafter; **never auto-run `npm run …`**.
- Rust: `cargo`. Any Python tooling: `uv`.

**How to use this document.** Work top-to-bottom. Each step has an **Objective**, **Actions**, and a **Verify** block. Do not start step *N+1* until step *N*'s Verify passes. Commit after each step.

---

## 1. Project scaffold (Tauri 2 + SvelteKit static)

**Objective.** A runnable empty Tauri 2 window that loads a SvelteKit app built with the static adapter.

**Actions.**
- [ ] Pick the frontend package manager (**pnpm** recommended). Record the choice at the top of `README.md`.
- [ ] Initialize the project with the Tauri 2 + SvelteKit template (`create-tauri-app`).
- [ ] Install `@sveltejs/adapter-static`; configure `svelte.config.js` with `adapter-static` and `prerender = true`.
- [ ] Configure `tauri.conf.json`: product name `EchoMind`, identifier `app.echomind`, dev URL matching Vite port.
- [ ] Add `.gitignore` entries for `node_modules`, `target`, `dist`, `.DS_Store`.

**Verify.**
- `pnpm tauri dev` opens a native window showing the default page.
- `pnpm tauri build` produces a bundle without errors (smoke test only).

---

## 2. UI foundation (Tailwind + shadcn-svelte + shadcn-ai-elements)

**Objective.** A themed app shell with working shadcn components.

**Actions.**
- [ ] Install Tailwind CSS v4 and wire it into SvelteKit (`app.css` with `@import "tailwindcss"`).
- [ ] Initialize `shadcn-svelte` (use the `shadcn-svelte` skill whenever adding components).
- [ ] Add base components: `Button`, `Card`, `Input`, `Sonner` (toasts).
- [ ] Install `shadcn-ai-elements` for streaming chat/thinking surfaces.
- [ ] Build the app shell in `src/routes/+layout.svelte`: top bar (title, record/stop button placeholder), main panel, status footer.
- [ ] Set up dark/light theming via Tailwind + a `mode-watcher` rune store.

**Verify.**
- Running dev shows the themed shell with a visible Button + Card.
- Toggling dark/light updates all components live.

---

## 3. i18n (English / Spanish)

**Objective.** All user-facing strings are localized and switchable at runtime.

**Actions.**
- [ ] Choose between `svelte-i18n` and a rune-based store (`$state` map keyed by locale). For a small app, prefer a rune store — simpler, fewer deps.
- [ ] Create `src/lib/i18n/en.ts` and `src/lib/i18n/es.ts` with keys for: app title, record/stop, meeting list, thinking-mode labels, errors, settings.
- [ ] Create `src/lib/i18n/index.svelte.ts` exporting a `locale` rune and a `t(key)` function derived via `$derived`.
- [ ] Persist the chosen locale in `localStorage`; default to the OS locale if it starts with `es`, otherwise `en`.
- [ ] Add a language toggle in the top bar.

**Verify.**
- Switching the toggle instantly swaps every string in the shell between English and Spanish.
- Reloading the app restores the last-chosen locale.

---

## 4. Rust backend skeleton

**Objective.** A clean module layout and a working Tauri command round-trip from the frontend.

**Actions.**
- [ ] Under `src-tauri/src/`, create modules: `audio/mod.rs`, `ai/mod.rs`, `storage/mod.rs`, `commands.rs`, `events.rs`, `error.rs`.
- [ ] In `error.rs` define a single `AppError` enum with `thiserror`; commands return `Result<T, AppError>`.
- [ ] In `events.rs` define string constants for every Tauri event emitted (`audio.level`, `transcript.partial`, `ai.token`, …).
- [ ] Add a `ping` command returning `"pong"` and register it in `main.rs`.
- [ ] On the frontend, create `src/lib/bridge.ts` wrapping `invoke` and `listen` with typed helpers.

**Verify.**
- The frontend calls `ping` and shows `"pong"` in the UI.
- `cargo check` and `pnpm check` both pass.

---

## 5. Audio capture (`cpal` loopback + mic)

**Objective.** Capture microphone input reliably and stream level data to the UI.

**Actions.**
- [ ] Add `cpal` and `ringbuf` to `src-tauri/Cargo.toml`.
- [ ] In `audio/`, build a `Recorder` that:
  - Enumerates input devices, picks the default mic (plus the selected device if provided).
  - Opens an input stream and pushes PCM frames into a lock-free `ringbuf`.
  - Emits `audio.level` events with RMS on a 50 ms cadence.
- [ ] Expose commands: `audio_list_devices`, `audio_start`, `audio_stop`.
- [ ] **System loopback note (macOS).** `cpal` cannot capture system audio natively on macOS; document the two options the user may enable later:
  - **BlackHole** virtual device (simplest, user installs and routes).
  - **ScreenCaptureKit** via a separate Rust crate (defer to a later sub-step if needed).
- [ ] Build a simple level meter in the UI using `$state` updated from the event listener.

**Verify.**
- Speaking into the mic moves the UI meter in real time.
- `audio_stop` releases the device (no leaking streams across start/stop cycles).

---

## 6. Transcription (Whisper via Candle — staging step)

**Objective.** Turn captured audio into live captions so later steps have text to feed Gemma.

**Actions.**
- [ ] Add `candle-core`, `candle-nn`, `candle-transformers`, and `tokenizers` to `Cargo.toml`. Use the Metal feature on macOS.
- [ ] Download a quantized Whisper small model (pin repo + revision in a constants file) into the platform cache dir.
- [ ] In `ai/whisper.rs`, load the model once at startup on a dedicated thread.
- [ ] Consume audio from the ring buffer in ~5 s windows with ~0.5 s overlap; run transcription off the audio callback thread.
- [ ] Emit `transcript.partial` and `transcript.final` events.
- [ ] In the UI, render live captions with `shadcn-ai-elements` message components.

**Verify.**
- Speaking produces live captions within ~1–2 s of speech.
- Captions stabilize (final) after the window closes.

---

## 7. Local Gemma 4 inference

**Objective.** Load Gemma 4 locally and stream tokens on demand.

**Actions.**
- [ ] Pick the Gemma 4 variant: **E2B** for low-RAM devices, **E4B** default. Pin the exact HF repo + revision in `ai/gemma.rs`.
- [ ] Use 4-bit quantization (GGUF or Candle's quantized tensors). Document the expected RAM usage in `README.md`.
- [ ] Build a `GemmaEngine` with `load()`, `generate(prompt, opts) -> Stream<Token>`, `cancel()`.
- [ ] Define the system prompt in `ai/prompts.rs` — "professional, unbiased meeting secretary" (from `GEMINI.md`), with placeholders for locale and meeting context.
- [ ] Expose commands: `ai_generate(prompt, locale)` and `ai_cancel()`; stream tokens via `ai.token` events.
- [ ] Ensure tokenization uses the matching Gemma tokenizer (`tokenizers` crate, not a hand-rolled BPE).

**Verify.**
- Sending a test prompt from the UI streams tokens smoothly into a `Card`.
- Cancel stops the stream within ~100 ms.

---

## 8. "Thinking Mode" streaming UI

**Objective.** A premium, reactive surface for Gemma's streamed output.

**Actions.**
- [ ] Build a `ThinkingPanel.svelte` that consumes `ai.token` events and appends to a `$state` buffer.
- [ ] Use `$derived` to compute rendered markdown (lean on `svelte-ai-elements`/`shadcn-ai-elements` message primitives).
- [ ] Use Svelte native transitions (`fly`, `fade`) for the overlay reveal/hide.
- [ ] Add a cancel button wired to `ai_cancel()`.

**Verify.**
- Streaming feels smooth (no jitter, no layout thrash) for multi-paragraph outputs.
- Cancel instantly halts the stream and leaves the partial text visible.

---

## 9. Meeting pipeline (transcript → Gemma)

**Objective.** Turn the live transcript into running summaries and action items.

**Actions.**
- [ ] Maintain a rolling transcript buffer in Rust with a size cap (e.g., last 10 minutes or N tokens).
- [ ] Trigger Gemma summarization on: explicit user button, ~20 s of silence, or every K new turns.
- [ ] Detect the dominant spoken language from the Whisper output; pass the locale into the Gemma system prompt so the response is in that language.
- [ ] Emit `summary.update` events with the latest summary + action items.
- [ ] UI: render a two-pane meeting view — live captions on the left, rolling summary + action items on the right.

**Verify.**
- A 60-second test meeting produces a coherent summary + ≥1 action item.
- Repeat in Spanish — output is in Spanish.

---

## 10. Persistence (SQLite)

**Objective.** Meetings survive restarts.

**Actions.**
- [ ] Choose `rusqlite` (simpler, sync) over `sqlx` unless async is needed — document the choice.
- [ ] Create schema in `storage/schema.sql`:
  - `meetings(id, started_at, ended_at, locale, title)`
  - `transcripts(id, meeting_id, t_start, t_end, text, is_final)`
  - `summaries(id, meeting_id, created_at, summary, action_items_json)`
  - `settings(key, value)` for locale, device, model choice.
- [ ] Run migrations at startup (idempotent `CREATE TABLE IF NOT EXISTS`).
- [ ] Expose CRUD commands: `meetings_list`, `meeting_get`, `meeting_create`, `meeting_append_transcript`, `meeting_save_summary`, `settings_get/set`.
- [ ] UI: Meeting History list + detail view.

**Verify.**
- Record a short meeting, close the app, reopen → history shows it with transcript + summary intact.

---

## 11. Overlay mode

**Objective.** A compact, always-on-top overlay usable during video calls.

**Actions.**
- [ ] In `tauri.conf.json`, define a second window `overlay` with: decorations off, always-on-top, transparent, skip taskbar.
- [ ] Add a `/overlay` SvelteKit route showing only: mic level, last 2 captions, summary ticker.
- [ ] Implement a click-through toggle (macOS: `setIgnoreCursorEvents`).
- [ ] Provide a hotkey to show/hide the overlay (Tauri global shortcut).

**Verify.**
- Overlay appears above Zoom/Meet and can be toggled click-through.
- Hotkey works when the main window is not focused.

---

## 12. Polish & release prep

**Objective.** A signed, launch-ready macOS build.

**Actions.**
- [ ] Add error toasts (Sonner) wired to a frontend `onError` handler and surface Rust `AppError` messages.
- [ ] Add file logging (`tracing` + `tracing-appender`) to a rotating file under the app support dir. **No network telemetry.**
- [ ] App icon set (iconset → `.icns`), bundle identifiers, minimum macOS version.
- [ ] Code-signing notes in `docs/signing.md` (Developer ID, notarization steps).
- [ ] Write a short `README.md` quickstart.

**Verify.**
- `pnpm tauri build` produces a `.dmg` that launches cleanly on a fresh user account.
- Logs appear in the expected path; no network traffic during a recorded session (confirm with Little Snitch or `lsof`).

---

## 13. Verification & testing

**How to run dev.**
- `pnpm tauri dev` — full app.
- `pnpm dev` — frontend only (for quick UI iteration with mocked bridge).

**Automated checks.**
- `cargo test --manifest-path src-tauri/Cargo.toml` — Rust unit tests (audio ring buffer, prompt assembly, schema migrations).
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`.
- `pnpm check` (svelte-check) and `pnpm lint`.

**Manual smoke test (must pass before each release).**
1. Launch the app, set language to English.
2. Start recording. Speak for ~30 seconds about a fake meeting.
3. Confirm: live captions appear, rolling summary updates, ≥1 action item is produced.
4. Stop recording. Close the app. Reopen.
5. Confirm: the meeting appears in history with transcript + summary intact.
6. Repeat steps 1–5 in Spanish.
7. Toggle overlay mode during a video call — confirm it stays on top and can go click-through.

---

## Appendix: decisions to lock early

- **Frontend package manager:** chosen in step 1, recorded in `README.md`.
- **Gemma variant:** E2B vs E4B, quantization format — chosen in step 7.
- **SQLite crate:** `rusqlite` vs `sqlx` — chosen in step 10.
- **System loopback strategy (macOS):** BlackHole vs ScreenCaptureKit — chosen in step 5 (or deferred).

Record each decision with a one-line rationale inside `README.md` under "Decisions" as you make it.
