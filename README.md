# Accolade

Accolade is a desktop app (SvelteKit + Tauri) that detects local achievement files, enriches them with Steam metadata, and shows live unlock notifications in an overlay.

## Features

- Local achievement scan from emulator files
- Real-time achievement updates with desktop overlay notifications
- Steam sync (profile, owned games, metadata)
- Optional SteamGridDB icon enrichment
- In-app settings for scan paths, HUD position, sound, theme, and accent color
- Setup page with language selection (FR/EN/ES/DE/IT)

## Supported emulators

- Goldberg
- Empress
- CODEX
- OnlineFix
- RUNE

## Prerequisites (Windows)

- Node.js 20+ and npm
- Rust toolchain (stable)
- Tauri v2 system requirements (MSVC Build Tools + WebView2)
- Tauri CLI:

```bash
cargo install tauri-cli --version "^2"
```

## Getting started

1. Install JavaScript dependencies:

```bash
npm install
```

2. Start the desktop app in development:

```bash
cargo tauri dev
```

3. In the app settings, set:
   - Steam ID (64-bit)
   - Steam API key (required for Steam sync)
   - SteamGridDB API key (optional)
   - Extra search paths (optional)

## Build

Frontend:

```bash
npm run build
```

Rust/Tauri check:

```bash
cargo check --manifest-path .\src-tauri\Cargo.toml
```

Desktop bundle:

```bash
cargo tauri build
```

## Optional environment variables

You can also provide API keys through environment variables:

- `STEAM_API_KEY`
- `STEAMGRIDDB_API_KEY`

These are used as fallback values when keys are not provided from settings.

## Default scan locations

| Emulator | Default location(s) |
| --- | --- |
| Goldberg | `%APPDATA%\Goldberg SteamEmu Saves` |
| Empress | `%APPDATA%\Empress-Emulator` |
| CODEX | `%APPDATA%\Steam\CODEX`, `%PUBLIC%\Documents\Steam\CODEX` |
| OnlineFix | `%PUBLIC%\Documents\OnlineFix` |
| RUNE | `%PUBLIC%\Documents\Steam\RUNE` |
