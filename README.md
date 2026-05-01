# Ebook Audio Studio

TypeScript full-stack app for turning ebooks into phone-friendly audio.

## Stack

- `pnpm` workspace
- React + Vite client
- Express + TypeScript server
- Local storage for uploads and generated audio
- Pluggable TTS provider

## Current Flow

1. Upload an ebook file from the web UI.
2. Pick a target language and narration tone.
3. Server extracts text and prepares it for narration.
4. TTS provider generates an audio artifact.
5. Browser shows a playable and downloadable result.

Text files, Markdown, and HTML can be processed immediately. PDF and EPUB have explicit hooks in `server/src/services/extractText.ts` for adding parsers.

## Local TTS

Whisper is not used for TTS because Whisper is speech-to-text. For offline text-to-speech on macOS, set:

```bash
TTS_PROVIDER=macos
```

The macOS provider uses `/usr/bin/say` and `/usr/bin/afconvert` to generate `.m4a` files.

## Development

```bash
pnpm install
cp server/.env.example server/.env
pnpm dev
```

Client: `http://localhost:5173`

Server: `http://localhost:4100`

## Environment

```bash
PORT=4100
PUBLIC_BASE_URL=http://localhost:4100
MAX_UPLOAD_MB=50
TTS_PROVIDER=mock
```

Use `TTS_PROVIDER=macos` for local macOS audio generation.

## GitHub Pages

The repository includes a GitHub Actions workflow that builds `client/` and deploys the static frontend to GitHub Pages.

GitHub Pages can host the React UI, but it cannot run the Express upload/TTS server. Deploy the server separately, then add a repository variable:

```bash
VITE_API_BASE_URL=https://your-api-host.example
```

When the Pages workflow builds the frontend, API calls and audio links will use that backend URL.
