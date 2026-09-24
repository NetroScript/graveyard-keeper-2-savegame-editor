# Graveyard Keeper 2 Save Editor

An unofficial save editor for **Graveyard Keeper 2**. It can edit player values,
manage supported inventories, undo changes, and inspect data that does not yet
have a dedicated editor.

The editor runs on your own device. The web version processes saves in your
browser and does not upload them to a server.

## Use the editor

### Web version

[Open the Graveyard Keeper 2 Save Editor](https://netroscript.github.io/graveyard-keeper-2-savegame-editor/)

Choose your `.dat` save and its matching `.info` file when available. The edited
save is downloaded as a new file, leaving the original untouched.

The web editor cannot create or manage backups in the game's save folder. Keep
the original `.dat` and matching `.info` files until you have confirmed the edited
save works in the game. Use the desktop version if you want save discovery,
automatic paired backups, and protection against overwriting a save changed by
the game or another program.

### Desktop version

Download the version for your operating system from the
[GitHub Releases page](https://github.com/NetroScript/graveyard-keeper-2-savegame-editor/releases).

The desktop application can find saves automatically. Before replacing a save,
it creates a compressed ZIP backup containing its `.dat` and `.info` files by
default; the number of retained backups is configurable in Settings. Earlier
versions can be restored from the save list. It also detects if another program
changed a save and can install signed application updates. On Windows, use the
installer if you want automatic updates. The standalone `.exe` can be run without
installation, but accepting an update will launch an installer.

The editor checks the save structure, but only the game can confirm that every
edited value is valid for a particular game version. Keep the desktop backup, or
your own backup when using the web editor, until the edited save has loaded
successfully.

## What can be edited

- Health, energy, stamina, money, happiness, insanity, and technology points.
- Player inventory, equipped tool belt, bags, chests, and supported world
  containers. Bags appear beneath the inventory that contains them.
- Item counts, quality variants, durability, capacity, insertion, replacement,
  and removal where the game rules are known.
- Technology trees, including dependency-aware unlocks and the game's rewards.
- Inspiration progress, unlocked levels, talent values, and perk trees.
- Raw save fields and structures through the Save Inspector.
- Undo and redo for accepted edits.

The side rail shows the game version represented by the included asset catalog.
If that version is older than the installed game, definitions or icons may be
outdated even when the save itself still opens.

## Finding saves

Typical release save locations are:

| Platform     | Location                                                                                                                            |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| Windows      | `%USERPROFILE%\AppData\LocalLow\Lazy Bear Games\Graveyard Keeper 2`                                                                 |
| macOS        | `~/Library/Application Support/Lazy Bear Games/Graveyard Keeper 2`                                                                  |
| Linux        | `~/.config/unity3d/Lazy Bear Games/Graveyard Keeper 2`                                                                              |
| Steam Proton | `~/.local/share/Steam/steamapps/compatdata/4358690/pfx/drive_c/users/steamuser/AppData/LocalLow/Lazy Bear Games/Graveyard Keeper 2` |

Steam libraries and Linux configuration directories can be stored elsewhere.

## For developers

The project uses Rust for lossless save processing, Svelte for the interface,
WebAssembly for browser builds, and Tauri for desktop builds.

Install Node.js, pnpm and Rust, then run:

```sh
pnpm install
rustup target add wasm32-unknown-unknown
pnpm dev
```

Useful commands:

| Command              | Result                                                           |
| -------------------- | ---------------------------------------------------------------- |
| `pnpm build:web`     | Static browser application in `build/web`                        |
| `pnpm tauri dev`     | Desktop development application                                  |
| `pnpm tauri build`   | Desktop executable and installers for the current platform       |
| `pnpm release:local` | Web and current-platform release files under `release/<version>` |
| `pnpm check`         | TypeScript and Svelte checks                                     |
| `pnpm test:core`     | Rust save-library tests                                          |
| `pnpm test:browser`  | Browser integration tests                                        |

More technical documentation:

- [Desktop and browser builds](docs/builds.md)
- [Release builds and updates](docs/releases.md)
- [Save library API](crates/save-core/README.md)
- [Game asset exporter](src-assets/README.md)

## License and game assets

The editor source code is available under the [MIT License](LICENSE).

The MIT License applies only to the editor's original source code. Graveyard
Keeper 2 and the game artwork and other game assets distributed with the editor
remain the property of Lazy Bear Games and their respective rights holders. No
ownership of, or license to, those assets is claimed, offered, or granted by this
project. Their inclusion does not imply authorization, affiliation, or
endorsement by Lazy Bear Games or any other rights holder.

If Lazy Bear Games or another applicable rights holder requests their removal,
the project maintainers will promptly remove the relevant assets from the hosted
web editor and from future downloadable distributions under their control.
