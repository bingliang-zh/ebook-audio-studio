# Ebook Audio Studio

Pure local Tauri desktop app for turning ebook text into audio with a user-owned TTS model.

## Architecture

- React + Vite UI
- Tauri v2 desktop shell
- Rust commands for local file access and process execution
- Piper-compatible local TTS generation
- No hosted backend, no API key, no project-owned server

## Current Flow

1. Choose a `.txt`, `.md`, or `.html` ebook file.
2. Choose the local `piper` executable.
3. Choose a local Piper `.onnx` voice model.
4. Choose where to save the output `.wav`.
5. Generate audio using the user's own CPU/GPU resources.

The app does not upload ebook text or audio anywhere.

## Requirements

- Node.js and pnpm for development
- Rust toolchain for Tauri
- A Piper executable installed locally
- A Piper `.onnx` voice model downloaded locally

Example setup:

```bash
pip install piper-tts
```

Then download a Piper voice model from the Piper voice model catalog and select it in the app.

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

- The language and tone controls are currently passed through to the local generation request as metadata. Actual language quality depends on the selected Piper model.
- PDF and EPUB support are not enabled yet. They can be added with local parsers in the Rust command layer.
