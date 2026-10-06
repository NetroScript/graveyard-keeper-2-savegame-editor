# Release history

This file contains the user-facing notes for each Graveyard Keeper 2 Save Editor
release. GitHub Releases use the matching version section automatically.

## [0.3.1] - Faith and science

- Added faith and science to the General view. The game stores them as items: faith in the player inventory, including bags, and science in the study table. The new fields are a shortcut for editing those item stacks, which can still be edited in the Inventory view. The editor shows each total and updates, adds or removes stacks of up to 999 to match the entered amount, as long as the inventory has free slots.
- Locking technologies and player perks that were unlocked when the save was opened now refunds the technology points or perk points they cost in the game. Unlocking them again costs those points again, so repeated locking and unlocking never adds points. Anything unlocked in the editor stays free and refunds nothing.
- Updated the bundled game data for Graveyard Keeper 2 version 1.009.1.

## [0.3.0] - Zombie editor

### Zombie editor

- Added a complete zombie worker editor with localized names, worker portraits, skull totals and available talent slots.
- Added editing for zombie names, technology points, organs, body treatments, body pockets, collars, armor and tools, with item artwork, skull values and game-aware slot choices.
- Added role-aware work storage. Porter cargo can be edited while preserving the game's reserved large-item slots; carried task items and workstation storage are identified for other worker roles.
- Added zombie talent-tree editing, including individual unlocks and automatic body and specialty optimization.
- Added an in-game-style appearance editor for supported bodies, heads and color palettes, with composited worker previews throughout the zombie editor.
- Added automatic incremental loading to zombie item pickers and clearer groups for treatments, skull-adding items, tools, armor and cargo.
- Cached zombie snapshots and applied incremental transaction updates so opening the editor, selecting workers and changing appearance no longer repeatedly reconstruct the entire zombie list.

### Technologies and perks

- Unlocked technologies and player perks can now be locked again by selecting them. Dependent technologies or perks are locked with them, and their crafts, buildings, formulas, active perks and mastery are removed unless something that stays unlocked still grants them.

### Notifications

- Added toast notifications for saves, settings changes and edit results, including a confirmation after each successful save. Notifications stack, pause while hovered or focused, and show their remaining time.
- Failed edits now appear as temporary notifications instead of banners that remained until the next edit, and no longer also show a second error banner above the editor.

### Desktop updates

- Added configurable desktop update checks with Never, Daily, and Weekly schedules. New and existing installations that have not chosen a schedule are prompted on startup, with Weekly preselected.
- Added a side-rail notification when a new application version is available.

### Game data

- Updated the bundled game data for Graveyard Keeper 2 version 1.007.
- Added extracted zombie customization, talent, perk and equipment data and sprites to the packaged game assets.
- Compressed the packaged game assets, reducing the web editor's asset download from about 5.4 MB to 1.9 MB.

## [0.2.0] - Save Inspector search

- Added simple and advanced Save Inspector search with field, class, type, substring, exact, wildcard, Boolean, path, reference and numeric-range matching. Structural child, descendant and parent predicates can return containers or elements based on related fields. Results include match explanations and a detailed syntax and examples dialog.
- Added a dismissible Save Inspector warning about backups and the risks of raw structural edits.
- Updated the bundled game data for Graveyard Keeper 2 version 1.006.

## [0.1.2] - Improvements and fixes

- Updated the bundled game data for Graveyard Keeper 2 version 1.005.
- Added custom track and thumb styling for Inspiration progress and inventory durability sliders, including WebKitGTK and Firefox.
- Prevented duplicate restore points when restoring the current version or switching between archived versions.
- Sized inline game icons to match surrounding text in progression descriptions.
- Kept Inspiration category pixel art sharp when switching tabs.
- Added a screenshot and description for web editor link previews.
- Added link to Reddit Profile to about page

## [0.1.1] - Linux download options

- Added `.deb` packages for Debian/Ubuntu and `.rpm` packages for Fedora. These use the system WebKitGTK instead of the AppImage's bundled WebKitGTK and forced X11 backend.
- Added an executable archive that uses system WebKitGTK for distributions such as Arch Linux. Extract the archive and run the executable; download a new archive to update it.
- On some NVIDIA Wayland systems, launch the archive's executable with `__NV_DISABLE_EXPLICIT_SYNC=1`. The same workaround can resolve the `Error 71` crash during desktop development.
- Kept the AppImage as a portable option. It may still feel laggy on some NVIDIA Wayland systems.
- Documented the Linux graphics workaround and the web editor and local build options when a download does not work.

## [0.1.0] - Initial release

The first public release of the unofficial Graveyard Keeper 2 Save Editor.

### Save editing

- Edit health, energy, stamina, money, happiness, insanity, and technology points.
- Manage the player inventory, equipped tool belt, bags, chests, and supported world containers with game-aware item rules.
- Change item counts, quality, durability, and container capacity; add, replace, repair, or remove supported items.
- Unlock technology trees with their dependencies and edit inspiration progress, levels, talents, and perk trees.
- Find and remove unwanted item drops from the world.
- Inspect unsupported save structures with the Save Inspector and undo or redo accepted edits.

### Desktop application

- Discover local saves and display their game metadata.
- Write saves safely with compressed rolling backups of matching `.dat` and `.info` files, including backup restoration and external-change detection.
- Check for signed application updates.

### Web application

- Edit saves locally in the browser without uploading them to a server.
- Download the edited save as a new file while leaving the selected original untouched.

### Platforms

- Web browsers with WebAssembly support.
- Windows, Linux, and macOS desktop builds.
- Portable Windows executables, compressed macOS application bundles for Intel and Apple Silicon, and Linux AppImages.

### Portable update behavior

- Accepting an update from the portable Windows executable launches the Windows installer. Download and replace the portable executable manually to keep using it without installation.
- Portable macOS application bundles and Linux AppImages can only replace themselves when they are stored in a location the current user can modify.
