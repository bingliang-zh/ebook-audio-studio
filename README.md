# Ebook Audio Studio

Pure local Tauri desktop app for turning ebook text into audio with a user-owned TTS model.

## Architecture

- React + Vite UI
- Tauri v2 desktop shell
- Rust commands for local file access and process execution
- Piper-compatible local TTS generation
- No hosted backend, no API key, no project-owned server

## Current Flow

1. Open the app.
2. Download one of the recommended built-in voice models.
3. Choose a `.txt`, `.md`, or `.html` ebook file.
4. Choose MP3 or WAV, then choose where to save the output audio.
5. Generate audio using the user's own machine.

The app does not upload ebook text or audio anywhere.

## Built-In Setup

The app includes a guided setup screen for non-technical users:

- Shows supported voice models as cards.
- Downloads the Piper engine for the user's current platform.
- Downloads an FFmpeg encoder when MP3 output is selected.
- Downloads the selected `.onnx` model and matching `.onnx.json` config into the app data directory.
- Reads speaker mappings from the model config when available.
- Auto-detects `piper` from the user's `PATH`.

Advanced users can open Settings to manually choose a Piper executable or a custom `.onnx` model.

## Requirements

- Node.js and pnpm for development
- Rust toolchain for Tauri
The app can download Piper and voice models from inside the setup flow. Manual Piper selection is available as a fallback.

## Development

```bash
pnpm install
pnpm dev
```

## Build

```bash
pnpm build
```

This creates a desktop bundle through Tauri.

## Notes

- Actual language quality depends on the selected Piper model.
- Tone is mapped to Piper speaking speed through `--length_scale`.
- Piper generates WAV internally. MP3 output is produced by converting that temporary WAV through FFmpeg.
- Preview generates a short temporary sample using the current model, speaker, tone, and output format.
- PDF and EPUB support are not enabled yet. They can be added with local parsers in the Rust command layer.
