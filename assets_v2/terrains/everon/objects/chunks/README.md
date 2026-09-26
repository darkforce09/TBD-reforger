# Everon object chunks

Everon's world objects, one file pair per populated 512 m cell of the island grid, plus the index
of which cells hold anything. The map engine streams these into the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s map as the camera moves.

## Contents

```text
assets_v2/terrains/everon/objects/chunks/
├── *.bin          one cell's objects as a `TBDC` container of 32-byte `ObjectInstancePod` rows
├── *.json.gz      the same cell's objects as gzip JSON rows, row for row the twin of its `.bin`
└── manifest.json  the chunk index: the cell size and every populated cell with its path and count
```

## How it works

The terrain manifest's `objects.chunksPath` names this folder and `objects.binary.chunks` names the
binary template `objects/chunks/{cx}_{cy}.bin`. The map engine's world loader fetches
`manifest.json` here, whose `cells` bound every chunk the residency requests, then
fetches each cell the viewport needs: the `.bin` when the manifest's `objects.binary` block
matches the container, version and row shape the build implements
(`ObjectsBinaryBlock::matches_this_build` in
`apps/website/map-engine/src/streaming/loaders/manifest.rs`), otherwise the `.json.gz`. Both
lanes build the same columns.

315 of the 625 cells of the 25 × 25 grid hold objects, 1,216,066 instances in all; an empty cell
has no files and no index row. Row `i` of a `.bin` and row `i` of its `.json.gz` are the same
instance: the writer derives the binary rows from the JSON it has just written, through the
loader's own narrowing.

## Format

- Encoding: files are named `{cx}_{cy}` after the cell's column and row on the 512 m grid.
  - `.bin`: a 32-byte little-endian `TBDC` header (magic, version 1, flags, row count, `cx`,
    `cy`, reserved bytes), then that many 32-byte rows and nothing else, so row `i` starts at
    byte `32 + 32·i`. A row holds `x`, `y`, `z`, `yaw`, `pitch`, `roll` and `scale` as `f32`,
    the prefab index as `u16`, the render class code and a zero pad byte. Stored in Git LFS.
  - `.json.gz`: gzip JSON `{"instances": [...]}`, each row `[prefabId, x, y, z, rotationDeg]` or,
    when pitch, roll or scale is non-trivial, `[prefabId, x, y, z, rotationDeg, pitchDeg,
    rollDeg, scale]`. Plain git blobs.
  - `manifest.json`: plain JSON, `chunkSizeM` (512) and `cells[]` of `cx`, `cy`, `path` and
    `instanceCount`.
- Schema: the rows follow `contracts_v2/definitions/map-object-instance.schema.json`, which also
  records the byte layout of the binary row; the container header is defined in
  `apps/website/map-engine/src/io/containers/tbdc.rs` and the row in
  `apps/website/map-engine/src/io/pod/instance.rs`. `prefabId` indexes the catalogue in the
  parent folder.
- Adding a file: never by hand. `cargo xtask map export-terrain everon --phase <phase>` rewrites
  the whole set; `cargo run -p developer-tools --bin world -- verify-phase --terrain everon --phase
  <phase>` checks it.

## Producers and consumers

- Producers: `world build-objects` (`tools_v2/developer-tools/src/world_export_pipeline/`, the
  binary rows in `binary_emit.rs`), which `cargo xtask map export-terrain` runs after its phase
  gate.
- Consumers:
  - the map engine's world loader and residency
    (`apps/website/map-engine/src/streaming/loaders/world_loader/`, `chunk_bin.rs`, `chunk.rs` and
    `residency.rs` in `apps/website/map-engine/src/streaming/loaders/`), fetched as
    `/map-assets/everon/objects/chunks/…`;
  - the developer tools' world line-of-sight verification and blueprint instance checks
    (`tools_v2/developer-tools/src/map_verification/world_line_of_sight/`,
    `tools_v2/developer-tools/src/blueprint/bvh/instance_verification/`);
  - tests that read the committed chunks: the map engine's chunk and residency tests
    (`everon_chunk_bin_columns_equal_the_gz_decode`, `ingest_chunk_bin_matches_ingest_chunk_gz`)
    and the world export's binary emission tests.

## Boundaries

- Depends on: the prefab catalogue `assets_v2/terrains/everon/objects/prefabs.json.gz` and its
  archive, whose indexes `prefabId` names; the manifest's `objects.binary` block.
- Used by: the map engine, the developer tools and the tests listed above.
- Rules: a `.bin` and its `.json.gz` always change together and decode to the same rows; a
  chunk's header names its own cell, and a loader refuses one that names another
  (`id_mismatch_is_rejected_even_though_the_bytes_are_perfect`); `manifest.json` lists exactly the
  cells that have files.

## Related documentation

- [Map binary formats](/apps/website/map-engine/src/io/README.md) — the `TBDC` container and the
  instance row.
- [World asset loaders](/apps/website/map-engine/src/streaming/loaders/README.md) — how the chunks
  are fetched, parsed and made resident.
