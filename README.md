# EchoMind

A private, open-source desktop app that acts as a real-time AI-powered meeting analyst. Audio never leaves your machine.

Built with **Tauri 2** + **Rust** on the backend and **Svelte 5** on the frontend, using a local **Gemma 4** model via the Candle framework.

## Features

- **System & mic audio capture** via `cpal`
- **Live transcription** via Whisper (local, quantized)
- **AI meeting analysis** — summaries and action items via local Gemma 4
- **Thinking Mode** — real-time streaming visualization of AI reasoning
- **Overlay mode** — compact always-on-top view for use during video calls
- **Meeting history** — persisted locally in SQLite
- **English / Spanish** — full i18n support

## Tech stack

| Layer | Technology |
|---|---|
| Frontend | Svelte 5 (Runes), SvelteKit, Tailwind CSS v4, shadcn-svelte, shadcn-ai-elements |
| Backend | Rust, Tauri 2 |
| AI / ML | Gemma 4 (local, 4-bit quantized), Whisper small (local), Candle |
| Storage | SQLite via rusqlite |
| Audio | cpal |

## Getting started

> Prerequisites: Rust (stable), pnpm, Xcode Command Line Tools (macOS).

```bash
# Install frontend dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build for release
pnpm tauri build
```

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Frontend package manager | pnpm | Fast, disk-efficient |
| Gemma variant | TBD (E2B / E4B) | Chosen in step 7 of the build plan |
| SQLite crate | rusqlite | Simpler sync API; no async overhead needed |
| macOS system loopback | TBD (BlackHole / ScreenCaptureKit) | Chosen in step 5 of the build plan |

## Build plan

See [`planning/PLAN.md`](planning/PLAN.md) for the full step-by-step implementation plan.

## Privacy

EchoMind runs entirely offline. No audio, transcripts, or summaries are ever sent to an external server.

## License

MIT
