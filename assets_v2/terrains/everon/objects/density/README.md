# Everon vegetation density tiles

Everon's tree and rock density at 8 m resolution, one small tile per 512 m cell of the island
grid. The map engine stitches them into one island texture that draws the forest fill and the
canopy in the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s map.

## Contents

```text
assets_v2/terrains/everon/objects/density/
└── *.bin  one cell's tree and rock density grid as a `TBDD` tile, named `{cx}_{cy}.bin`
```

## Format

- Encoding: 625 tiles, one for every cell of the 25 × 25 grid of 512 m cells, empty ones included,
  named `{cx}_{cy}.bin` like the object chunks. Each is 16,916 bytes: a 16-byte little-endian
  `TBDD` header (magic, version 1, cell size 8 m, 65 columns, 65 rows, 2 channels), then two
  planes of 65 × 65 `u16` values, row-major, `tree` then `rock`. The 65 corners of a 512 m cell
  share their border corners with the next cell. The tree plane is the canopy-blurred grid, not the
  raw count. Plain git blobs, not Git LFS: the last rule of `.gitattributes` exempts this folder
  from the `*.bin` LFS rule.
- Schema: the codec is `apps/website/map-engine/src/io/density/tbdd.rs`; the terrain manifest's
  `objects.densityPath` and `objects.densityCellM` (8) describe the folder.
- Adding a file: never by hand. `world build-objects` rewrites the set on a density phase (P2 and
  up) and leaves it alone on any other; `cargo run -p developer-tools --bin world -- redensify
  --terrain everon` rebuilds it from the committed object chunks alone.

## Producers and consumers

- Producers: `world build-objects` and `world redensify`, in
  `tools_v2/developer-tools/src/world_export_pipeline/chunk_partitioner/`, which encode with
  `encode_tbdd`; `cargo xtask map export-terrain` runs the first.
- Consumers:
  - the map engine's vegetation loader
    (`apps/website/map-engine/src/world/environment/vegetation/loader.rs`), which fetches all 625
    tiles as `/map-assets/everon/objects/density/{cx}_{cy}.bin`, skips one that fails to decode,
    and uploads the stitched tree plane once;
  - the density codec's tests, which decode every committed tile against a byte-by-byte reference
    (`everon_tiles_decode_bit_identically_to_the_old_loop` in
    `apps/website/map-engine/src/io/density/tests/tbdd_tests.rs`), and the world export's
    vegetation density tests.

## Boundaries

- Depends on: the object chunks in `assets_v2/terrains/everon/objects/chunks/`, whose trees and
  rocks the tiles count.
- Used by: the map engine's vegetation loader and the tests listed above.
- Rules: the tiles are plain git blobs, so the `.gitattributes` exception stays that file's last
  line, after the `*.bin` LFS rule it overrides; the loader builds each path from the terrain id
  and the grid rather than from `densityPath`, so the file names and the 625-tile grid are fixed.

## Related documentation

- [Vegetation density tiles](/apps/website/map-engine/src/io/density/README.md) — the `TBDD`
  layout and its decoder.
- [Vegetation](/apps/website/map-engine/src/world/environment/vegetation/README.md) — how the tiles
  become the forest fill and canopy.
