# Workspace API and extensions

`gk2-save-core::Workspace` owns every authoritative save document. Both the Tauri state and the WebAssembly worker use this implementation. Components retain presentation state and drafts only.

## Document lifecycle and transactions

Opening bytes returns a summary with `documentId`, `revision`, byte counts, `dirty`, `canUndo` and `canRedo`. Every subsequent `request(documentId, command)` or `export(documentId)` targets that document. Handles are monotonically allocated record indices: reordering, deleting, undoing and inserting never assigns an old handle to a different node. They are session handles, not file offsets or persistent IDs.

Queries include `summary`, `children` (offset/limit, maximum 200), `nodes` (up to 200 handles), `general`, `drops` and `templates`. Close releases a document. Numeric field values cross transport boundaries as strings; input/output files use binary buffers.

```json
{
  "op": "transact",
  "revision": 3,
  "operations": [
    { "op": "set", "node": 24, "tag": 31, "value": "1.25" },
    { "op": "set", "node": 25, "tag": 31, "value": "2.5" }
  ]
}
```

A transaction journals the original version of each touched record and commits all operations together. Errors restore that journal and discard appended records. It returns `summary`, `changed` and `deleted`. Stale revisions and invalid operations reject the entire batch. UI mutations are queued per document and still carry the revision captured when submitted. Undo/redo increments revisions and applies the same record patches. History is limited to 100 transactions or 64 MiB of retained record patches per document; a transaction larger than that memory budget is applied without retained undo history.

All edit types use this incremental path, including inventory removal/replacement and structural Inspector edits. Reverse-reference and GUID indexes support local checks, collection counts update only on touched arrays, and dirty tracking rehashes touched records and their ancestors. There is no document clone, full-document comparison, serialization or full-tree validation during a transaction or undo/redo. Initial inventory discovery remains a separate cached query; raw Inspector edits invalidate that cache for the next inventory visit. Large subtree edits still cost work proportional to the affected records and ancestor child lists.

`summary.encodedBytes` is a number on open and after export, and `null` after an edit or undo/redo until the next export. This avoids calculating serialized size just for the summary. Export encodes the document, checks the file size limit and validates the resulting wire data before returning bytes. Local field, reference, depth, record-count and inventory constraints are checked during editing; the total serialized byte limit is checked on export.

For a read-only native debug benchmark against a local save and packed catalog, run `node scripts/benchmark-native.mjs <save.dat> [game.gk2pack]`. It times edits and undo/redo separately and verifies byte-identical restoration without writing the save. `cargo run -p gk2-save-core --example benchmark_edits -- <save.dat>` runs a smaller set without a catalog. These measure core requests, excluding IPC and rendering. Rebuild WASM before running `node scripts/benchmark-inventory.mjs <save.dat>` for the browser codec.

Operations: `set`, `insert`, `template`, `duplicate`, `remove`, `move`, `retarget`, `general`, `drops`. `move.index` and insertion indices are zero-based sibling positions. Arrays require unnamed entries. Retarget uses a destination **node handle** declaring an Odin object, not the object's serialized ID. Removal of referenced objects is rejected unless the same batch resolves all affected references. Cloning allocates fresh Odin IDs, remaps references inside the cloned subtree and preserves references to outside objects. SGuid and other game identifiers are unchanged. These checks establish serialization integrity, not all game rules.

The encoder traverses the current containment tree, updates array counts and relocates a type declaration if its original node is removed or reordered. Unrelated payload bytes and original type-name string encodings remain intact. Unedited exports and undo back to the original document are byte-identical. Removed records remain inaccessible but allocated so their handles and raw type definitions are not reused.

## General selectors

`general` resolves named fields from `playerData.hpComponent` and the paired `playerData.res.resType` / `resValues` lists. It validates health as int32 and resource values as float32. Duplicate names, differing list lengths, mismatched atom names and incorrect value types disable resource controls rather than guessing.

`general` operations accept named values (`hp`, `max_hp`, `money`, `energy`, `stamina`, `insanity`, `happiness`, `tech_red`, `tech_green`, `tech_blue`). Missing supported resources are inserted into both lists in one transaction using the registered GameResAtom template. Inputs must be finite and nonnegative; maximum health must be positive. Untouched abnormal values are retained. Float values displayed after Apply reflect the actual accepted float32 value. Money is stored in base units; the UI edits it as gold/silver/bronze using 10,000/100/1 and submits one base-unit value.

New game classes or fields do not require schema regeneration for raw preservation. Named General selectors tolerate unrelated additions and ordering changes. Renamed fields, changed layouts or scalar types may require selector updates and otherwise produce explicit unavailable controls. A type matcher is not a blanket guarantee of compatibility with every future game version.

The `drops` query finds ordinary `DropData` entries in every scene's active and queued drop lists. It returns stable handles together with the item ID, count, drop type, source list, world and position. A `drops` operation accepts a list of those handles, verifies that every selection is still an ordinary drop and removes them atomically. The General tab uses this for selective cleanup; the transaction can be undone like any other edit. Dedicated technology-point drops are left alone because current game versions already collect visible and viewless points through their collect-all routine.

## Add an Inspector widget

Place a `*.registration.ts` module alongside an optional Svelte component in `src/lib/inspector/widgets/`. The module's default export satisfies `Registration`: stable `id`, numeric `priority`, and one or both of an editor (`matches` plus `component`) and a `tree(node)` presentation. Vite's eager build-time glob discovers registrations automatically; no central list changes are needed. Restart/rebuild the frontend after adding a module.

Matchers must validate both the full type/assembly identity and the actual child layout. Highest priority wins; tied top matches deliberately fall back to the generic Inspector. The built-in vectors handle verified Unity Vector2/3/4 and Vector2Int/3Int layouts, including unnamed positional scalars. They reject partial or incompatible layouts. Components receive `node`, `children` and `transact(operations)` and must apply grouped changes atomically. They do not receive mutable Rust state. Show raw structure remains available even when a widget matches.

`tree(node)` can add a colored type label and labeled summary values to the corresponding tree row. A type label is omitted when it repeats the property name. It may set `compact` to hide ordinary child expansion, as the vector and SGuid registrations do. Selecting a hidden raw child from the details pane temporarily expands its parent so tree and details selection remain synchronized. `node.treeFields` contains the node's direct scalar children as read-only display data; it avoids an IPC request for every visible tree row. Tree presentations only affect display and never bypass transactions.

The SGuid widget presents its serialized `id` as one GUID field and validates the canonical hyphenated representation before submitting a string-field transaction. Its internal representation stays hidden unless the user explicitly enables the raw structure view.

## Add a complex-value template

Place a JSON file in `crates/save-core/templates/`. Cargo's build script scans that directory and embeds templates into native and WebAssembly builds automatically. Rebuild both codecs after adding one.

```json
{
  "id": "example-vector",
  "typeName": "UnityEngine.Vector2, UnityEngine.CoreModule",
  "reference": false,
  "fields": [
    { "kind": "f32", "value": "0" },
    { "kind": "f32", "value": "0" }
  ]
}
```

Fields may have `name`; omitted names represent positional Odin values. Supported kinds are the scalar kinds exposed by Inspector. A `template` operation supplies `parent`, `index`, optional `name`, `template` ID and a `values` map of field-name (or positional-index) overrides. Templates create only known registered layouts, with a fresh Odin object ID for reference types. The initial template format describes scalar-member objects, not an arbitrary constructor for unknown classes. More complex gameplay-aware templates should extend the Rust validation/building layer and have explicit invariant tests before registration.

## Desktop persistence

`gk2-save-desktop` isolates path discovery, metadata and file writes from the codec. Native discovery reads environment-derived Unity directories, Steam library configuration and known Proton application prefixes. Tests supply synthetic environments and directories. Metadata is parsed for display only; original `.info` bytes travel with the source and are copied unchanged by Save As.

Files are fingerprinted when opened and checked before replacement. A conflict returns `EXTERNAL_CHANGE`; the UI offers Reload, Save As and Cancel. Saves use synced temporary files in the destination filesystem and a `.filename.editor-recovery` journal holding the previous pair. Incomplete replacements roll back on recovery; a committed marker prevents cleanup interruptions from rolling back a successful save. Editor backups are compressed ZIP archives beneath `.gk2-editor-backups/<destination filename>/save-*.zip`; each archive contains the previous `.dat` and matching `.info` when present. Retention is pruned only after successful replacement. Filesystem cleanup failures may leave additional recoverable backups. As with ordinary file editors, another process writing during the final OS replacement window cannot be coordinated without that process cooperating; close the game before acceptance testing edited saves.

Settings live in Tauri's application configuration directory or browser local storage. Workspace documents are not restored after restart. Browser saving downloads copies and optional matching sidecars; it does not discover folders or create automatic backups.
