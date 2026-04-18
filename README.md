# EchoMind

Private, AI-powered meeting analyst for macOS. Captures microphone audio, transcribes it locally with Whisper, and generates real-time summaries and action items using a local Gemma 3 model — no data ever leaves your machine.

Built with [Tauri 2](https://tauri.app) + [SvelteKit](https://kit.svelte.dev) + [Candle](https://github.com/huggingface/candle).

---

## Features

- **Live transcription** — Whisper Small runs on-device via Metal (macOS GPU)
- **AI summaries** — Gemma 3 1B streams meeting summaries and action items in real time
- **Meeting history** — all transcripts and summaries are persisted locally in SQLite
- **English / Spanish** — UI and AI output both support switching at runtime
- **100% private** — no network calls during a session beyond the one-time model download

---

## Requirements

- macOS 13+ (Apple Silicon or Intel)
- [Rust](https://rustup.rs) stable
- [pnpm](https://pnpm.io) 9+
- A Hugging Face account with access to the Gemma 3 model (see setup below)

---

## Setup

### 1. Install dependencies

```bash
pnpm install
```

### 2. Obtain a Hugging Face token

Gemma 3 is a gated model. You need to:

1. Create a free account at [huggingface.co](https://huggingface.co)
2. Accept the license at [huggingface.co/google/gemma-3-1b-it](https://huggingface.co/google/gemma-3-1b-it)
3. Generate an access token at [huggingface.co/settings/tokens](https://huggingface.co/settings/tokens) (read access is enough)

### 3. Create `.env`

Create a `.env` file in the project root:

```
HF_TOKEN=hf_your_token_here
```

### 4. Run in development

```bash
pnpm tauri dev
```

On first recording start, the app will download the Whisper Small (~250 MB) and Gemma 3 1B (~5 GB) model weights into the Hugging Face cache (`~/.cache/huggingface`). This happens once; subsequent starts load from the local cache.

---

## Usage

1. Click **Start Recording** — the button shows "Loading models…" on first run while the models download
2. Speak; live captions appear in the left pane within ~1–2 s
3. The AI generates a rolling summary in the right pane automatically (after ~30 s of silence or every 2 minutes)
4. Click **Manual Summary** at any time to trigger a summary immediately
5. Click **Stop Recording** to end the session — the meeting is saved to history
6. Switch to the **History** tab to browse past meetings and reload their transcripts and summaries

---

## System audio (macOS loopback)

`cpal` cannot capture system audio natively on macOS. To transcribe the far end of a call, install [BlackHole](https://existential.audio/blackhole/) and create a Multi-Output Device in Audio MIDI Setup that routes to both your speakers and BlackHole, then select BlackHole as the input device in EchoMind (device selector coming in a future step).

---

## Development

```bash
pnpm tauri dev        # full app with hot-reload frontend
pnpm dev              # frontend only (Vite, no Tauri bridge)
pnpm check            # svelte-check + TypeScript
cargo clippy          # Rust lints (run from src-tauri/)
```

---

## Architecture

```
src/                  SvelteKit frontend (Svelte 5 Runes)
  routes/+page.svelte Main meeting UI
  lib/bridge.ts       Typed wrappers for Tauri invoke + listen

src-tauri/src/
  audio/              cpal capture → ringbuf → rubato resampler → 16 kHz chunks
  ai/
    whisper.rs        Whisper Small encoder-decoder (Candle, Metal)
    gemma.rs          Gemma 3 1B inference (Candle, Metal)
    meeting.rs        In-memory transcript buffer + activity tracking
    prompts.rs        Gemma 3 chat-template prompt builder
  storage/            rusqlite — meetings, transcripts, summaries, settings
  commands.rs         Tauri IPC handlers
  events.rs           Event name constants
```

---

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Frontend package manager | **pnpm** | Fast, strict hoisting, lockfile stability |
| AI model | **google/gemma-3-1b-it** via `candle_transformers::models::gemma3` | Smallest instruction-tuned Gemma generation with full candle 0.9 support; text-only, fits the summarisation task |
| Quantization | F32 (unquantized) for now | Candle's quantized gemma3 path is available (`quantized_gemma3` module) — switchable once the basic pipeline is stable |
| SQLite crate | **rusqlite** (bundled) | Simpler sync API; no async overhead needed for infrequent storage writes |
| macOS loopback | **BlackHole** (user-installed) | ScreenCaptureKit requires entitlements and a more complex setup; BlackHole is one install away and well-understood |

---

## Roadmap

- [ ] Device selector UI (wire up `audio_list_devices`)
- [ ] Overlay mode (always-on-top compact window with captions ticker)
- [ ] 4-bit quantization for Gemma 3 (reduce ~5 GB → ~1 GB RAM)
- [ ] Language detection from Whisper output → auto-locale for summaries
- [ ] Proper streaming detokenizer (handle byte-fallback UTF-8 fragments)
- [ ] Code signing + notarization for distribution
