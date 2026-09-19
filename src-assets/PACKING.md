# Application asset packs

The exporter produces local JSON and PNG source files. A separate packer builds
one binary file for the application. The plugin does not need to run again when
changing the application or its outline colors.

Requires Node 22.18+ (native TypeScript support) or Node 24, and `pnpm install`.

```sh
pnpm pack:assets "<export-directory>"
pnpm test:assets
pnpm test:assets:browser
```

The default output is `static/assets/game.gk2pack`. An optional second argument
sets another output path. Files in `static` are copied into both frontend builds;
generate the pack before building the application. Raw dumps and loose PNGs
should stay outside `static`. Generated packs are ignored by Git. CI can obtain
the versioned pack as an input artifact without installing or launching the game.

The browser test uses an installed Chrome by default. Set `PLAYWRIGHT_CHANNEL`
to another installed Playwright browser channel if needed. It verifies actual
pixel output, a shared canvas, cached requests and URL cleanup using synthetic
images; game files are not required for the tests.

Validation failures stop packing. `--allow-partial` permits an explicitly partial
catalog for development and retains its diagnostics as `catalogs.packWarnings`.
Invalid image hashes, paths or binary structures are still rejected. A partial
catalog must not be treated as authoritative for inventory insertion rules.

## Binary format, version 1

| Offset         | Contents                                                    |
| -------------- | ----------------------------------------------------------- |
| 0              | Eight bytes: `GK2PACK` followed by a zero byte              |
| 8              | Version, unsigned 32-bit little endian                      |
| 12             | MessagePack metadata length, unsigned 32-bit little endian  |
| 16             | MessagePack metadata: `schemaVersion`, `catalogs`, `images` |
| After metadata | Concatenated PNG byte payloads                              |

Image entries contain `offset` relative to the PNG payload start, `length`,
`width`, and `height`, keyed by SHA-256. Shared images occur once. Source filenames
are removed from the image index. All top-level JSON catalogs are included,
including future exporter modules. Packing is deterministic for identical input.
Controller font icons are removed and unused images are omitted.
PNG retains lossless compression; metadata uses MessagePack.

## Browser and Tauri rendering

`src/lib/assets/asset-pack.ts` exports `AssetPack`, which provides catalog access
and image rendering backed by the shared parser:

```ts
import { asset } from "$app/paths";
import { AssetPack } from "$lib/assets/asset-pack";

const assets = await AssetPack.load(asset("/assets/game.gk2pack"));
// Resolve imageHash through the item catalog and its named sprite entry.
const url = await assets.imageUrl(imageHash, "#272832");
// Use url as the src of a normal <img>. Repeated requests reuse the same URL.
// Omit the color for TMP icons and other images without blue-key replacement.
// Call assets.dispose() when this pack is no longer used by any displayed images.
```

The renderer lazily creates one offscreen HTML canvas (not one per item). Work
is serialized through it, and generated Blob URLs are cached by image hash and
outline color. The DOM uses ordinary image elements. Concurrent identical
requests share the same promise. Disposal revokes URLs and rejects queued work;
callers should release the `AssetPack` instance to release the original pack
buffer too. Limit the number of requested outline colors to the UI palette to
keep the cache small. No filesystem images are generated at runtime.

The replacement rule currently targets exactly RGB `(0, 0, 255)` and preserves
alpha.
