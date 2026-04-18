# EchoMind: Private Meeting Intelligence

**EchoMind** is an open-source desktop application built with **Tauri** and **Rust** that serves as a real-time, AI-powered meeting analyst. It leverages Google’s **Gemma 4** model locally for 100% data privacy.

## ✨ Core Features

1.  **System Audio Loopback:** Capture system and mic audio via Rust (`cpal`).
2.  **Local Gemma 4 Inference:** Agéntic reasoning using the `Candle` framework.
3.  **High-Performance UI:** Built with **Svelte** for ultra-fast interactions.
4.  **AI Streaming:** Real-time "Thinking Mode" visualization via `svelte-ai-elements`.
5.  **Multi-language (i18n):** Native support for English and Spanish.

## 🛠️ Tech Stack

- **Frontend:** Svelte 5 (Runes) + Tailwind CSS + shadcn-svelte + shadcn-ai-elements.
- **Backend:** Rust (Tauri).
- **AI Engine:** Gemma 4 (Local).
- **Storage:** SQLite for persistent meeting history.
