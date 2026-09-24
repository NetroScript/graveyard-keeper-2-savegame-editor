# Release history

This file contains the user-facing notes for each Graveyard Keeper 2 Save Editor
release. GitHub Releases use the matching version section automatically.

## [0.1.0] - Initial release

The first public release of the unofficial Graveyard Keeper 2 Save Editor.

### Save editing

- Edit health, energy, stamina, money, happiness, insanity, and technology
  points.
- Manage the player inventory, equipped tool belt, bags, chests, and supported
  world containers with game-aware item rules.
- Change item counts, quality, durability, and container capacity; add, replace,
  repair, or remove supported items.
- Unlock technology trees with their dependencies and edit inspiration progress,
  levels, talents, and perk trees.
- Find and remove unwanted item drops from the world.
- Inspect unsupported save structures with the Save Inspector and undo or redo
  accepted edits.

### Desktop application

- Discover local saves and display their game metadata.
- Write saves safely with compressed rolling backups of matching `.dat` and
  `.info` files, including backup restoration and external-change detection.
- Check for signed application updates.

### Web application

- Edit saves locally in the browser without uploading them to a server.
- Download the edited save as a new file while leaving the selected original
  untouched.

### Platforms

- Web browsers with WebAssembly support.
- Windows, Linux, and macOS desktop builds.
