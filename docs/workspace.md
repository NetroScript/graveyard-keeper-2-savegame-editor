# Workspace API and extensions

`gk2-save-core::Workspace` owns every authoritative save document. Both the Tauri state and the WebAssembly worker use this implementation. Components retain presentation state and drafts only.

## Document lifecycle and transactions

Opening bytes returns a summary with `documentId`, `revision`, byte counts, `dirty`, `canUndo` and `canRedo`. Every subsequent `request(documentId, command)` or `export(documentId)` targets that document. Handles are monotonically allocated record indices: reordering, deleting, undoing and inserting never assigns an old handle to a different node. They are session handles, not file offsets or persistent IDs.

Queries include `summary`, `children` (offset/limit, maximum 200), `nodes` (up to 200 handles), `general` and `templates`. Close releases a document. Numeric field values cross transport boundaries as strings; input/output files use binary buffers.

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

A transaction validates against a private candidate and commits all operations together. It returns `summary`, `changed` and `deleted`. Stale revisions and invalid operations reject the entire batch. UI mutations are queued per document and still carry the revision captured when submitted. Undo/redo increments revisions and acts on complete transactions. History is limited to 100 transactions or 64 MiB of retained record patches per document; a transaction larger than that memory budget is applied without retained undo history.

Operations: `set`, `insert`, `template`, `duplicate`, `remove`, `move`, `retarget`, `general`. `move.index` and insertion indices are zero-based sibling positions. Arrays require unnamed entries. Retarget uses a destination **node handle** declaring an Odin object, not the object's serialized ID. Removal of referenced objects is rejected unless the same batch resolves all affected references. Cloning allocates fresh Odin IDs, remaps references inside the cloned subtree and preserves references to outside objects. SGuid and other game identifiers are unchanged. These checks establish serialization integrity, not all game rules.

The encoder traverses the current containment tree, updates array counts and relocates a type declaration if its original node is removed or reordered. Unrelated payload bytes and original type-name string encodings remain intact. Unedited exports and undo back to the original document are byte-identical. Removed records remain inaccessible but allocated so their handles and raw type definitions are not reused.

## General selectors

`general` resolves named fields from `playerData.hpComponent` and the paired `playerData.res.resType` / `resValues` lists. It validates health as int32 and resource values as float32. Duplicate names, differing list lengths, mismatched atom names and incorrect value types disable resource controls rather than guessing.

`general` operations accept named values (`hp`, `max_hp`, `money`, `energy`, `stamina`, `insanity`, `happiness`, `tech_red`, `tech_green`, `tech_blue`). Missing supported resources are inserted into both lists in one transaction using the registered GameResAtom template. Inputs must be finite and nonnegative; maximum health must be positive. Untouched abnormal values are retained. Float values displayed after Apply reflect the actual accepted float32 value. Money is stored in base units; gold/silver/bronze are 10,000/100/1.

New game classes or fields do not require schema regeneration for raw preservation. Named General selectors tolerate unrelated additions and ordering changes. Renamed fields, changed layouts or scalar types may require selector updates and otherwise produce explicit unavailable controls. A type matcher is not a blanket guarantee of compatibility with every future game version.

## Add an Inspector widget

Place a `*.registration.ts` module alongside a Svelte component in `src/lib/inspector/widgets/`. The module's default export satisfies `Registration`: stable `id`, numeric `priority`, `matches(node, children)` and `component`. Vite's eager build-time glob discovers registrations automatically; no central list changes are needed. Restart/rebuild the frontend after adding a module.

Matchers must validate both the full type/assembly identity and the actual child layout. Highest priority wins; tied top matches deliberately fall back to the generic Inspector. The built-in vectors handle verified Unity Vector2/3/4 and Vector2Int/3Int layouts, including unnamed positional scalars. They reject partial or incompatible layouts. Components receive `node`, `children` and `transact(operations)` and must apply grouped changes atomically. They do not receive mutable Rust state. Show raw structure remains available even when a widget matches.

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

Files are fingerprinted when opened and checked before replacement. A conflict returns `EXTERNAL_CHANGE`; the UI offers Reload, Save As and Cancel. Saves use synced temporary files in the destination filesystem and a `.filename.editor-recovery` journal holding the previous pair. Incomplete replacements roll back on recovery; a committed marker prevents cleanup interruptions from rolling back a successful save. Editor backups are paired files beneath `.gk2-editor-backups/<destination filename>/save-*`. Retention is pruned only after successful replacement. Filesystem cleanup failures may leave additional recoverable backups. As with ordinary file editors, another process writing during the final OS replacement window cannot be coordinated without that process cooperating; close the game before acceptance testing edited saves.

Settings live in Tauri's application configuration directory or browser local storage. Workspace documents are not restored after restart. Browser saving downloads copies and optional matching sidecars; it does not discover folders or create automatic backups.
