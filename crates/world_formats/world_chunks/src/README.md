# World chunks source

The source of `world_chunks`: the two chunk decoders, the chunk identifier, the terrain manifest,
the crate's error and prelude, and the `TBDC` fixtures other crates' tests share.

## Contents

```text
crates/world_formats/world_chunks/src/
├── chunk_container.rs   the `TBDC` container decoder, its identity check and the path template
├── chunk_id.rs          `ChunkId`, the `cx_cy` identifier of a chunk
├── error.rs             `Error` and `Result`: the container errors, wrapped
├── lib.rs               the crate root: module header, `mod` lines and the re-exports
├── prelude.rs           the common names for glob import
├── terrain_manifest.rs  the manifest's objects block, binary blocks and chunk index cells
├── test_fixtures.rs     `TBDC` encoding and column comparison, behind `cfg(test)` or `test_fixtures`
├── tests/               unit tests of every module
└── world_chunk.rs       `WorldChunk`, the column form, and the chunk JSON decoder
```

## How it works

`world_chunk` defines the column form; `chunk_container` fills the same form from a container;
`terrain_manifest` stands alone; `chunk_id` names chunks for all three.

## Boundaries

- Depends on: `prefab_catalog`, `world_file_formats`, `map_coordinates`, `newtype_ids`, `serde`,
  `serde_json`, `bytemuck`, `thiserror`.
- Used by: the map engine and the developer tools, through the crate root.
- Rules: `lib.rs` holds only the module header, `mod` lines and `pub use` lines
  (`cargo xtask verify crate-anatomy`).
