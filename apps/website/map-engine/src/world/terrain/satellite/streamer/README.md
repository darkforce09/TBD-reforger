# Satellite container format

The reader of a terrain's satellite container, the `.tbd-sat` file that holds the whole satellite
image as a mip pyramid of WebP tiles: its header, its tile index in two versions, the checks an
index must pass, and the choice of the base and preview levels. It is plain computation; the
browser loading lives in the sibling `quadtree/` folder.

## Contents

```text
apps/website/map-engine/src/world/terrain/satellite/streamer/
├── archive.rs     version 2: the mip chain and tile rectangles derived from the archived index
├── header.rs      the index's end byte from a 12-byte prefix, and the header and index parse
├── mod.rs         the module tree; re-exports the index types, the parsers and the level picks
├── model.rs       `TbdSatIndex`, its levels and tiles, `TbdSatError`, and the format constants
├── selection.rs   the base level for a texture limit and the preview level for an edge size
├── t935_10/       unit tests that hold both container versions to the same answer
└── validation.rs  the loose and strict index checks
```

## How it works

A container starts with the bytes `TBDS` and a version:

```text
version 1  magic u32 | version u32 = 1 | JSON length u32 | JSON index | tile payload
           the JSON gives every tile's x, y, width, height and absolute offset and length
version 2  32-byte TbdsHeader (crate::io::containers::tbds) | rkyv TbdSatIndexV2 | tile payload
           the index gives the base size, tile_px, and per level the tile grid and each tile's
           offset from the payload start, length and format
```

`parse_header` reads either into one `TbdSatIndex` and stamps the container version on it;
version 2 carries no terrain id or world bounds, which read as absent. For version 2,
`mips_from_archive` derives what the index leaves out: the mip chain (`mip_dims`: each level half
the one before, rounded down, at least 1, down to 1 × 1) and every tile's rectangle on its level's
`tile_px` grid (`tile_rect`); a level count, grid, tile count or tile format other than WebP that
disagrees is refused. `index_range_end` sizes the header and index from the first 12 bytes, so a
loader fetches the index with a second Range request; an index over 16 MiB is refused.

`parse_tbd_sat_index_only` checks that the index's format version matches its container, that the
base size and level count are sane, and that every tile's bytes lie in the file after the index.
`parse_tbd_sat_index_strict` also requires levels numbered from 0 in order, each exactly half the
one before, tiles inside their level that cover it exactly, and a chain that ends at 1 × 1.
`pick_base_level` returns the first level whose long edge fits a texture limit, and
`pick_preview_level` the first whose long edge fits a preview size; each falls back to the last
level.

Everon's container, `assets_v2/terrains/everon/satellite/everon-sat.tbd-sat`, is version 1: a
12 800 × 12 800 base in 14 levels, about 153 MB, stored in Git LFS. The `build-unified` command of
the `map` binary (`tools_v2/developer-tools/src/map_raster_pipeline/`) writes either version.

## Public surface

- `TbdSatIndex`, `TbdSatMip`, `TbdSatTile` and `TbdSatError`.
- `index_range_end` and `parse_header`; `parse_tbd_sat_index_only` and
  `parse_tbd_sat_index_strict`.
- `pick_base_level`, `pick_base_level_for_limit` and `pick_preview_level`.

## Boundaries

- Depends on: `crate::io::containers` (the `TBDS` header) and `crate::io::archives` (the archived
  index and its validation); `serde_json` for the version 1 index.
- Used by: `crate::world::terrain::satellite::quadtree`, which reads the index and picks the
  levels; and the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s tests in
  `apps/website/frontend/src/v2/apps/editor/tests/`, which parse Everon's index and check the level
  choice.
- Rules: the folder compiles only with the `streaming` feature; both container versions of one
  pyramid parse to the same tiles at the same bytes (`v1_and_v2_of_one_pyramid_read_back_identical`
  in `t935_10/cases_1.rs`), and a version 2 index that contradicts its own grid is refused
  (`a_grid_that_contradicts_tile_px_is_rejected`).

## Related documentation

- [Everon dataset](/assets_v2/terrains/everon/README.md) — the terrain files, the satellite
  container among them.
