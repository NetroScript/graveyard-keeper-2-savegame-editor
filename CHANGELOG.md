# Release history

This file contains the user-facing notes for each Graveyard Keeper 2 Save Editor
release. GitHub Releases use the matching version section automatically.

## [0.1.2] - Unreleased

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
