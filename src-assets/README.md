# Graveyard Keeper 2 asset exporter

BepInEx plugin for exporting the installed Graveyard Keeper 2 game's item catalog,
English localization, technology trees, inspirations, perks, inventory compatibility
data, sprites, quality overlays, and TextMesh Pro sprite icons. Exports are JSON and shared PNG images,
usable by either the native or browser editor without Unity or game DLLs.

## Build and installation

This project targets **BepInEx 5's Unity Mono API** and .NET Standard 2.1.
Use the Unity Mono loader. BepInEx 6 requires adapting the plugin's loader API.
Rebuild against your installed game assemblies after game updates.

1. Install the appropriate **BepInEx 5 x64 Unity Mono** distribution beside the
   game's executable, following the [official installation guide](https://docs.bepinex.dev/articles/user_guide/installation/index.html).
   Launch once and check `BepInEx/LogOutput.log` for successful startup, then exit.
2. Install a .NET SDK (the project was compiled with SDK 8).
3. Build from the repository root, supplying the real Managed directory:

   ```sh
   dotnet build src-assets/Gk2.AssetExporter.csproj -c Release -p:GameManagedDir="<game-directory>/<executable-name>_Data/Managed"
   ```

   Use the `_Data/Managed` folder beside your game executable; its name depends
   on the installed edition. Alternatively create ignored `src-assets/Paths.local.props`:

   ```xml
   <Project>
     <PropertyGroup>
       <GameManagedDir>YOUR_GAME_MANAGED_DIRECTORY</GameManagedDir>
     </PropertyGroup>
   </Project>
   ```

   References use the installed game assemblies, not copied/decompiled source.
   NuGet supplies the pinned BepInEx development API. Only the plugin DLL needs
   deployment; do not copy Unity, game, or build dependency DLLs over the game.

4. Copy `src-assets/bin/Release/netstandard2.1/Gk2.AssetExporter.dll` into
   `BepInEx/plugins/Gk2.AssetExporter/`.
5. Launch the game. After reaching the menu, load a save and open the inventory
   once so scene-specific UI assets are loaded. Press **F8** to export. It yields
   between items and glyphs, but atlas loading/readback can briefly pause a frame.
6. Look in `BepInEx/asset-exports/<timestamp>-<run-id>/`. The plugin logs the exact
   directory and writes `manifest.json` **last**. A directory without a manifest
   is unfinished. `status: partial` includes explicit missing assets/errors.

The generated `BepInEx/config/gk2.saveeditor.assetexporter.cfg` configures the
shortcut and output directory. Relative output paths are relative to BepInEx.
Each export gets a new directory; previous runs are retained. No save files are
written and the user's language selection is not changed.

The [setup tutorial](https://docs.bepinex.dev/master/articles/dev_guide/plugin_tutorial/1_setup.html)
and [plugin tutorial](https://docs.bepinex.dev/master/articles/dev_guide/plugin_tutorial/2_plugin_start.html)
describe the development workflow. Their current examples also cover BepInEx 6;
use the BepInEx 5 API for this project.

## Export contents

| File                          | Contents                                                                                                                                              |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `manifest.json`               | Schema/exporter/game/Unity versions, assembly MVID, completed modules, warnings                                                                       |
| `item-ids.json`               | Every loaded `GameBalance.itemDefs` ID, including separate quality variants                                                                           |
| `items.json`                  | Definitions keyed by ID, English name/description, sprite links, quality metadata, effect expressions, related perk icons, complete definition fields |
| `item-definition-schema.json` | Exported field names and C# types; numeric/name mappings for item type and size enums                                                                 |
| `quality-families.json`       | Star-quality variants grouped by the game's base-ID rule                                                                                              |
| `inventory-rules.json`        | Per-bag allowed item IDs and per-item equipment eligibility                                                                                           |
| `resources.json`              | All resource-to-icon mappings for every configured icon type                                                                                          |
| `progression.json`            | Technology tabs/nodes/dependencies/rewards and talent branches, inspiration levels, perk-tree nodes, costs, text and icon links                       |
| `icons.json`                  | Named sprites, TMP sprite characters/metrics/assets, and shared image index                                                                           |
| `localization.en.json`        | English resource text with nested/replacement markup preserved                                                                                        |
| `images/<sha256>.png`         | Deduplicated RGBA images; identical images are written once                                                                                           |

`items[id].fields` contains every public instance field, including inherited
`id`, plus private `[SerializeField]` fields such as `customIcon`. It includes
`itemGroupIds`, `bagItemGroups`, `isBag`, `bagSizeX/Y`, `bagSize`, `inventorySize`,
`itemSize`, `stackCount`, `type`, durability, prices, flags, talents, runes, skulls,
and all use/drop/buy/sell expressions. Internal caches and executable methods are
not copied. Enum fields use names, with numeric mappings in the schema where
needed. Unknown future field objects are marked `unsupportedType` rather than
silently evaluated or recursively traversed.

Names follow `ItemDef.GetHeader`'s key selection: a star variant first uses its
own ID, then its base ID if no translation exists. Descriptions use the same
fallback with `_d`. Localization retains game markup; consumers must render only
supported tags and escape other content, not insert arbitrary HTML. Nested
`#(locale-key)` and dynamic `@(replacement-key)` tokens remain explicit.

## Inventory validation

The exporter calls the inspected, data-only `CanBeInsertedInBag` for every
item/bag pair. Bags cannot be inserted in bags, and an ordinary
item is accepted when its `itemGroupIds` intersects the destination's
`bagItemGroups`. An empty allowed-group list accepts nothing. Each bag result
has `complete`; a failed check is reported, never treated as allowed.

It also exports `CanItemBeEquipped()` and `IsFightingEquipment()` results, plus
the actual item type. This avoids inferring eligibility from display names or
assuming contiguous enum values. Equipment slot compatibility still requires
the slot's accepted type.

**Definition compatibility is not sufficient to validate an insertion.** The
inventory editor must also read the saved destination and check:

- `WhiteListFilterSerializedItemProperty` and `BlackListFilterSerializedItemProperty`:
  a nonempty whitelist ID list must contain the exact ID, and a nonempty
  whitelist group list must intersect the item's groups (both restrictions
  apply). A blacklist rejects an ID match **or** a group match. Empty whitelist
  lists impose no restriction; empty blacklist lists reject nothing.
- Actual `inventorySize` and occupied slots, including compatible nested bags.
- Existing stacks with the same exact ID and remaining `stackCount` capacity.
- `AutoExpandSerializedItemProperty` where applicable
- Instance state such as unique IDs, bag ownership, and equipment slot type.

Different quality IDs are distinct stack identities. A known ID means the game
has a definition, not that it is a normal obtainable item or valid in every
container. Preserve unknown IDs in existing saves; restrict adding new items to
known definitions whose relevant checks can be completed. Export again after
game updates and match the manifest to the save/game version.

## Icon sources and quality

`UIItemCell` looks up `ItemDef.iconId` through `EasySpritesCollection`. For
`qualityType == Star`, it adds a separate `item_star_<quality>` sprite. Quality
families use the portion of the ID before `:`, matching `GameBalance`'s cache.
No fixed number of quality levels is assumed.

`ItemDef.qualityIcon` is a separate **tooltip effect/rating icon**, not the star
overlay. Tooltips can also show energy, resource changes, weapon/armor ratings,
talent bonuses/requirements, skulls, runes and perks. Their definitions and icon
links are exported without executing effect expressions. `CanBeUsed` and some
other properties evaluate conditions against runtime state; raw expressions
are retained instead of presenting one save's evaluated values as universal.

`FontIcon()` returns `<sprite name="...">`. The exporter enumerates the default sprite asset, its
fallbacks, resources at TMP's configured sprite path, loaded text components,
and loaded sprite assets. It crops each glyph from the texture and records its
metrics. Assets that only load in other scenes or separate addressable bundles may require opening those screens and exporting again; required unresolved names are
reported in the manifest. Duplicate names across assets retain all candidates.

Resource icon types are Common=0, MoneyBig=1, and
TechPointSmall=2. `resources.json` exports the actual table. Resolve `gld`, `slv`,
`brz`, energy, stamina, insanity, happiness, and technology resource names
through that table. Direct tags such as `tech_red`, `rune_r`, `rune_g`, `rune_b`
also appear in `icons.fontIcons`. Health may use a dedicated UI image rather
than a resource mapping; an unavailable icon is reported.

To display an inventory item, resolve `items[id].sprite` into
`icons.sprites[name].image`, then `icons.images[hash].path`. Draw its optional
quality overlay separately. For a resource, resolve its configured icon name
into `icons.fontIcons[name]` and choose the appropriate asset candidate; then
resolve the shared image hash in the same way.

The sprite exporter uses mesh vertices and UVs, preserving transparent margins
and handling trimmed/rotated atlas entries. GPU readback supports non-readable
textures. It restores render state and releases temporary Unity objects. The
image cache retains only one source atlas at a time.

The inventory UI module also exports loaded `UIItemCell` background sprites to
`inventory-ui.json`. Open the character inventory before exporting so the cell
prefab is loaded, then rebuild the asset pack. Older packs use the editor's CSS
cell background until this catalog is available.

## Validation and extending the exporter

Compilation has been verified against the Unity 6000.3.9 Mono assemblies
using .NET SDK 8.

```sh
node --test src-assets/validate-export.test.mjs
node src-assets/validate-export.mjs "<export-directory>"
```

The validator checks catalog coverage, schema fields, quality references, bag
rules, icon/image links, PNG dimensions and hashes. It exits nonzero for partial
exports. Review warning details and visually compare representative sprites,
stars and resource icons with the game; a successful build does not verify GPU
colors, runtime loading, or gameplay compatibility.
