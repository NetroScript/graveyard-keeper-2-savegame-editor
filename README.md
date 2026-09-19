# Graveyard Keeper 2 save editor

A lossless Rust save library and Svelte inspector, available as a Tauri desktop application or a browser-only WebAssembly build. A demo save has passed byte-identical reconstruction in native Rust, WebAssembly and the browser UI.

## Run

Install Node, pnpm and Rust, then `pnpm install`. For browser builds, run `rustup target add wasm32-unknown-unknown` once. The project includes wasm-pack as a development dependency; its first run downloads the matching tools.

| Command | Result |
| --- | --- |
| `pnpm dev` | Browser development server; builds WebAssembly first |
| `pnpm build:web` | Static browser site in `build/web` |
| `pnpm preview` | Preview the browser build |
| `pnpm tauri dev` | Desktop development application |
| `pnpm tauri build` | Desktop executable and configured installers |
| `pnpm tauri build --no-bundle` | Desktop executable without installer packaging |
| `pnpm test:core` | Rust codec and service tests |
| `pnpm test:wasm` | Test the generated WebAssembly artifact; build it first |
| `pnpm test:browser` | Build and test the browser UI with Playwright Chromium |

Desktop builds require the normal platform-specific Tauri prerequisites. `pnpm dev:desktop` starts only the frontend for Tauri; use `pnpm tauri dev` to launch both processes.

Before running browser tests, install their browser with `pnpm exec playwright install chromium`.

Open a `.dat` file, browse the record tree and export a copy. Integer, finite float, boolean and string values can be edited in memory. This is a raw inspector: game-specific validators, collection insertion/removal, undo, and `.info` metadata updates are not implemented yet. No-op export preserves the original bytes; edited saves still require in-game validation.

## File formats

The inspected demo already writes `GameSave` to `.dat` using Odin's **binary** format (`DataFormat.Binary`). Embedded UTF-16 field names and strings can look readable in a text editor, but the surrounding tokens, lengths and values are binary. The current library reads and writes this format, with byte-identical unedited exports.

The companion `.info` file is separate JSON slot metadata, written by `JsonFileSerializer`. The editor does not currently load or modify it. It also does not implement Odin's JSON/text serializer. These are separate formats, not text and binary modes that the game switches between for the same save payload. Future changes to the binary grammar or encryption would require additional codec support.

## Documentation

- [Builds and shared backend API](docs/builds.md)
- [Library API](crates/save-core/README.md)
- [Item definitions and icon exporter](src-assets/README.md)
