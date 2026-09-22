# Release builds

Release builds require a packed `game.gk2pack`. The pack is deliberately excluded
from Git, then included in the generated web and desktop applications. The
workflow reads it from a separate draft GitHub release because GitHub Actions
manual inputs do not support uploading a file. The draft is only an input handoff;
the resulting public builds may distribute the pack normally.

## One-time setup

1. Keep `.tauri-private-key` safe. It signs updater bundles and is ignored by Git.
   Losing it prevents existing desktop installations from accepting future
   updates.
2. Add its complete contents as the repository secret
   `TAURI_SIGNING_PRIVATE_KEY`:

   ```powershell
   Get-Content -Raw .tauri-private-key | gh secret set TAURI_SIGNING_PRIVATE_KEY
   ```

3. In the repository Pages settings, choose **GitHub Actions** as the source.
4. Platform trust signing is separate from updater signing. The workflow creates
   ad-hoc-signed macOS builds. Configure Apple notarization and Windows code
   signing secrets before presenting those downloads as trusted installers.

## Provide the private asset input

Pack the export locally, then stage it on a draft release:

```powershell
pnpm pack:assets "C:\path\to\asset-export" --allow-partial
pnpm release:stage-assets
```

The second command creates or updates the draft release tagged
`asset-pack-input`. Keeping this input release as a draft prevents it from being
mistaken for an application release. Delete it after a release if desired and
recreate it before the next build.

## Build without publishing

Run **Build release** from the Actions tab with `publish` disabled. The workflow
builds the WebAssembly site plus Windows x64, Linux x64, macOS Apple Silicon and
macOS Intel desktop bundles. Each result is retained as a private workflow
artifact. It does not deploy Pages or create a public GitHub release.

For a current-platform local build:

```powershell
pnpm release:local -- --asset "C:\path\to\game.gk2pack"
```

This writes the web application and the current platform's desktop bundles under
`release/<version>/`. Native desktop installers must be built on their target OS;
the GitHub Actions matrix supplies those operating systems.

## Publish a release

Push a version tag matching the application version, for example `v0.1.0`, or run
the manual workflow with `publish` enabled. A publishing run deploys the web build
to GitHub Pages and creates a GitHub Release with desktop installers, updater
signatures and `latest.json`. The packed game asset remains on the separate draft
input release and is not attached to the public release as a standalone file. It
is embedded in desktop applications and served as part of the public web build.

## Installed and standalone desktop builds

The standalone Windows executable can be run directly. Checking for updates also
works there, but applying a Windows update launches the signed NSIS installer and
exits the running application. In practice, accepting the update installs the
application, so users who want automatic updates should start with the installer.
Users who want a portable executable can keep downloading and replacing it
manually instead.

Linux updates use the AppImage, while macOS updates replace the application
bundle. Those platforms require the user to restart the application after an
update is installed.

Keep the version in `package.json`, `src-tauri/tauri.conf.json` and
`src-tauri/Cargo.toml` synchronized before tagging.
