# Satellite imagery

The `satellite_imagery` crate: the reader of a terrain's satellite container, the `.tbd-sat` file
that holds the whole satellite image as a mip pyramid of WebP tiles. It reads the container's
header and its tile index in two versions, checks the index, and picks the base and preview
levels. It is plain computation; the browser loads and the texture uploads stay in the map
engine's `world/terrain/satellite/` module.

## Contents

```text
crates/terrain/satellite_imagery/
├── Cargo.toml  the package: `world_file_formats`, `serde`, `serde_json`, layout tier 2
└── src/        the header, both index versions, the index checks and the level picks
```

## How it works

A container starts with the bytes `TBDS` and a version:

```text
version 1  magic u32 | version u32 = 1 | JSON length u32 | JSON index | tile payload
           the JSON gives every tile's x, y, width, height and absolute offset and length
version 2  32-byte TbdsHeader (world_file_formats::containers::tbds) | rkyv TbdSatIndexV2 | tile payload
           the index gives the base size, tile_px, and per level the tile grid and each tile's
           offset from the payload start, length and format
```

`parse_header` reads either into one `TbdSatIndex` and stamps the container version on it;
version 2 carries no terrain id or world bounds, which read as absent. For version 2 the reader
derives what the index leaves out: the mip chain (each level half the one before, rounded down, at
least 1, down to 1 × 1) and every tile's rectangle on its level's `tile_px` grid; a level count,
grid, tile count or tile format other than WebP that disagrees is refused. `index_range_end` sizes
the header and index from the first 12 bytes, so a loader fetches the index with a second Range
request; an index over 16 MiB is refused.

`parse_tbd_sat_index_only` checks that the index's format version matches its container, that the
base size and level count are sane, and that every tile's bytes lie in the file after the index.
`parse_tbd_sat_index_strict` also requires levels numbered from 0 in order, each exactly half the
one before, tiles inside their level that cover it exactly, and a chain that ends at 1 × 1.
`pick_base_level` returns the first level whose long edge fits a texture limit, and
`pick_preview_level` the first whose long edge fits a preview size; each falls back to the last
level.

Everon's container, `assets/terrains/everon/satellite/everon-sat.tbd-sat`, is version 1: a
12 800 × 12 800 base in 14 levels, about 153 MB, stored in Git LFS. The `build-unified` command of
the `map` binary (`tools/map_assets/map_raster_pipeline/src/`) writes either version.

## Getting started

Run from the repository root:

```bash
cargo test -p satellite_imagery   # both container versions, the refusals and Everon's tiling
```

## Public surface

- `TbdSatIndex`, `TbdSatMip`, `TbdSatTile` and `TbdSatError`.
- `index_range_end` and `parse_header`; `parse_tbd_sat_index_only` and
  `parse_tbd_sat_index_strict`.
- `pick_base_level`, `pick_base_level_for_limit` and `pick_preview_level`.
- `Error` and `Result` (`error`), and `prelude`, which re-exports the items above.

## Boundaries

- Depends on: `world_file_formats` (the `TBDS` header, the archived version 2 index and its
  validation, the `TerrainId` of a version 1 index); `serde` and `serde_json` for the version 1
  index; `thiserror`.
- Used by: `map_asset_loading`'s satellite loader
  (`crates/streaming/map_asset_loading/src/terrain/satellite_quadtree/`),
  which reads the index and picks the levels; and the
  [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s tests in
  `apps/frontend/src/workspaces/editor/tests/`, which parse Everon's index and check the level
  choice.
- Rules: both container versions of one pyramid parse to the same tiles at the same bytes
  (`v1_and_v2_of_one_pyramid_read_back_identical` in `src/tests/satellite_container_tests.rs`),
  and a version 2 index that contradicts its own grid is refused
  (`a_grid_that_contradicts_tile_px_is_rejected`); terrain tier 2 (`cargo xtask verify
  crate-tiers`).

## Related documentation

- [Everon dataset](/assets/terrains/everon/README.md) — the terrain files, the satellite
  container among them.
