# gk2-save-core

A platform-independent, game-scoped Odin binary reader/writer. It contains no filesystem access, game assemblies, .NET runtime, Tauri or browser dependencies. The implementation reconstructs parsed records; it does not return a cached original input buffer.

```rust
use gk2_save_core::Document;

fn round_trip(input: &[u8]) -> Result<Vec<u8>, gk2_save_core::Error> {
    let document = Document::decode(input)?;
    Ok(document.encode())
}
```

For inspection/editing use `Service::default()`, `open(&bytes)`, `request(Request::Children { parent, offset, limit })`, `request(Request::SetValue { edit })`, and `export()`. `Request`/`Response` are the shared Serde protocol for native and WebAssembly adapters. `Close` releases the document. Failed loads and edits leave the existing document intact.

`NodeView.id` is a session-local record index; it is not a semantic field identifier and must not be persisted across saves. `original_offset` is an input-file offset. Edit requests require the expected wire tag and current revision, preventing stale/type-mismatched edits. A successful load or edit advances the revision. Integers cross JSON as decimal strings, including i64/u64; original floating-point bits remain untouched unless edited. Non-finite float edits, char/decimal/GUID changes, reference edits and container edits are currently read-only. Editing a string replaces its code units; untouched strings retain even unpaired surrogates and their encoding choice.

The default parser limits are 128 MiB input, one million records and 256 nested containers. `Document::decode_with_limits` allows callers to choose their limits. In-memory storage is larger than the input because it includes owned payloads and indexes. Unknown game type names are supported without loading their classes. Unknown wire tokens, invalid lengths, dangling references, duplicate IDs, mismatched arrays, trailing data and incomplete documents return errors. Encryption/compression and `.info` metadata are outside this initial codec.

The current parser accepts one complete root value, including synthetic primitive roots used in tests. A higher-level game editor should additionally require the expected `GameSave` root/schema before exposing game-specific controls.

## Local fixture verification

```powershell
cargo run -p gk2-save-core --example verify -- "./private/save.dat"
```

This reads the fixture, reconstructs it, compares every byte, toggles an existing root boolean in memory, checks that exactly one byte changed, restores it and compares again. It writes no files. The example expects a game save with a root boolean; it is not a generic command-line editor.

The code was written for this project from the investigated wire grammar; it does not include decompiled game code or translated Odin implementation routines. Bringing upstream implementation code or extracted game data into the project requires a separate review of its applicable license and distribution rights.
