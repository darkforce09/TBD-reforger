# World chunks

The `world_chunks` crate: a terrain's world object chunks and its terrain manifest as the map reads
them. A chunk is one cell of the world object grid, served as gzip JSON or as a `TBDC` binary
container; both decode into the same columns. The terrain manifest says where every served map
file is and how it is encoded.

## Contents

```text
crates/world_formats/world_chunks/
├── Cargo.toml  the package: `prefab_catalog`, `world_file_formats`, `map_coordinates`, `newtype_ids`, the dev-only `test_fixtures` feature, tier 3
└── src/        the chunk decoders, the chunk identifier and the terrain manifest
```

## How it works

```text
manifest.json, chunks/manifest.json ──> terrain_manifest: objects and binary blocks, index cells
chunks/<cx>_<cy>.bin ─────> chunk_container::parse_chunk_bin_for ──┐
chunks/<cx>_<cy>.json.gz ─> world_chunk::parse_chunk (+ prefab map) ┴─> WorldChunk
```

A `WorldChunk` holds one chunk's instances as columns (interleaved x and y, prefab id, yaw, z,
pitch, roll, scale, class code) plus the rows of each render class, and both decoders build the
same columns. The JSON lane joins each row's prefab id against `prefab_catalog`'s prefab map for
its render class; the binary lane reads the class from the row. A `TBDC` chunk is a header and
32-byte `ObjectInstancePod` rows: a payload whose length disagrees with the header's count is an
error, a misaligned buffer is copied into aligned words first, and `parse_chunk_bin_for` refuses a
well-formed chunk whose header names another chunk than the requested `ChunkId`. A `ChunkId` is
`cx_cy`, spelled by the chunk grid's `map_coordinates::chunk_math::chunk_id`.

The manifest parser never fails on a binary block: a missing or malformed block is absent, and
`ObjectsBinaryBlock::matches_this_build` decides whether chunk binaries are read. The `objects`
block needs `prefabsPath` and `chunksPath`; `chunkSizeM` defaults to `DEFAULT_CHUNK_SIZE_M`
(512 m). `chunk_bin_path` fills the binary block's `objects/chunks/{cx}_{cy}.bin` template.

## Getting started

Run from the repository root:

```bash
cargo test -p world_chunks   # both chunk lanes over the Everon export, the manifest, the identifier
```

The container test that compares both lanes over every Everon chunk reads
`assets/terrains/everon/objects/chunks/*.json.gz` and the committed Everon catalogue.

## Configuration

One feature, `test_fixtures`, off by default: it compiles the `test_fixtures` module (`TBDC`
encoding of a chunk and a bit-for-bit column comparison) for the tests of other crates and is
enabled only from their `[dev-dependencies]`. The crate reads no environment variable.

## Public surface

- `world_chunk`: `WorldChunk`, `parse_chunk`.
- `chunk_container`: `parse_chunk_bin`, `parse_chunk_bin_for`, `chunk_bin_path`, `ChunkBinError`.
- `chunk_id::ChunkId`, also at the crate root.
- `terrain_manifest`: `parse_manifest_binary` with `ManifestBinary` and its blocks,
  `parse_objects_manifest` with `ObjectsManifest`, `narrow_cells` with `ChunkCell`,
  `satellite_unified_encoding`, `DEFAULT_CHUNK_SIZE_M`, `TBDC_CONTAINER`,
  `SAT_UNIFIED_ENCODING_V2`.
- `Error` and `Result` at the crate root; the common names in `prelude`; `test_fixtures` under
  `cfg(test)` or the `test_fixtures` feature.

## Boundaries

- Depends on: `prefab_catalog` (the prefab map, `NO_CLASS`, the instance row narrowing),
  `world_file_formats` (the `TBDC` header, `ObjectInstancePod`, `BinaryError`), `map_coordinates`
  (`chunk_id`), `newtype_ids`, `serde`, `serde_json`, `bytemuck`, `thiserror`.
- Used by: `world_store`, `vegetation` and `world_line_of_sight`; the streaming crates
  `chunk_scheduler`, `chunk_draw_buffers`, `map_streaming_host` and `map_asset_loading` (the world
  loader and the elevation, label and water loaders); the world export and map checks in
  `tools/developer_tools/src/`.
- Rules:
  - world formats category, tier 3 (`cargo xtask verify crate-tiers`);
  - the binary and JSON lanes build the same columns
    (`everon_chunk_bin_columns_equal_the_gz_decode`);
  - a truncated or mis-served container is an error, never a short chunk
    (`truncated_payload_is_err_not_a_short_chunk`,
    `id_mismatch_is_rejected_even_though_the_bytes_are_perfect`);
  - the committed Everon manifest parses unchanged (`everon_manifest_parses_unchanged`);
  - a `ChunkId` serialises exactly as its string (`chunk_id_serialises_exactly_as_its_string`).

## Related documentation

- [Terrain manifest schema](/contracts/definitions/terrain-manifest.schema.json) — the manifest
  this crate narrows.
