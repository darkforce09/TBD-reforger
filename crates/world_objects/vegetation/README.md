# Vegetation

The `vegetation` crate: how the 2D map shows vegetation. The forest mass, a density grid of trees
across the whole island with its outline traced from the same grid; the tree counts that decide
between drawing each tree and letting the forest mass stand in; and the land-cover regions
(forest, field, water body) of the terrain. The browser loader and the GPU lane that upload the
forest mass stay in the map engine.

## Contents

```text
crates/world_objects/vegetation/
├── Cargo.toml  the package: `world_file_formats`, `world_chunks`, `prefab_catalog`, layout tier 4
└── src/        the regions, the forest mass, the tree counts and the density grid
```

## How it works

The map engine's `ForestMassHost` builds the forest mass once per terrain. It fetches the 625 density bins
`objects/density/{cx}_{cy}.bin` from the terrain's asset folder (`assets/terrains/everon/` for
Everon, served under `/map-assets/`), twelve at a time with up to three attempts each, and
stitches each bin's 65 × 65 tree-count corners into one `ISLAND_CORNERS` × `ISLAND_CORNERS`
(1601) grid of 8 m cells, neighbouring chunks sharing their border corners. Only when every bin
has arrived does it upload the grid as one texture, each corner's count (up to 255) in the red
channel and north as row 0, through `forest_density_upload`, and trace the forest outline where
the grid crosses `CANOPY_MASS_ISO` (2 trees) by marching squares into hairlines. After that, each
frame changes only the fill's opacity (`forest_fill_alpha`: 0.45 below zoom −2.5, 0.35 up to
zoom 1, 0.12 up to zoom 3, none beyond) and whether the fill and the outline show at that zoom.

`canopy.rs` counts the tree and vegetation rows of the chunks being drawn, exactly or weighted by
how much of each chunk the viewport covers; `heatmap_trees` reports when the count passes
`INSTANCE_BUDGET` (150 000), and the streaming packer then stops packing one glyph per tree and
lets the forest density fill stand in for them. The map draws no tree as geometry of its own: a
tree is a glyph or part of the forest mass.

`parse_regions_payload` narrows `objects/forest-regions.json.gz` to `LandCoverRegion`s: an id, a
kind (`forest`, `field` or `waterBody`), polygon rings (the first the outline, the rest holes) and
optional tree statistics; a row without an id, with another kind or with a malformed ring is
dropped. `regions_from_bytes` reads the same regions from `objects/forest-regions.rkyv`, and
`region_to_archive` writes one.

The grid dimensions are Everon's: `CHUNKS_PER_AXIS` (25) chunks of 512 m a side,
`EVERON_DENSITY_BINS` (625) bins, and the 12 800 m world the loader places the texture over.

## Getting started

Run from the repository root:

```bash
cargo test -p vegetation   # tree counts, density grid, marching squares and regions tests
```

## Public surface

- `regions`: `LandCoverRegion` (its id a `ForestRegionId`), `parse_regions_payload`,
  `region_to_archive` and `regions_from_bytes`.
- `mass`: `ForestMassGeometry`, `forest_mass_from_corners`,
  `forest_outline_segments_from_corners`, `forest_fill_alpha` and `CANOPY_MASS_ISO`.
- `canopy`: `exact_tree_count`, `visible_tree_count`, `heatmap_trees` and the density grid packing.
- `density`: the island constants, `stitch_chunk_into_island` and the texture packers.
- `Error` and `Result` (`error`), and `prelude`, which re-exports the items above.

## Boundaries

- Depends on: `world_file_formats` (the forest archive and `ForestRegionId`); `world_chunks` (the
  chunk rows the counts sum); `prefab_catalog` (the tree and vegetation class codes);
  `map_draw_lanes` (the instance budget and the class zoom gates); `map_coordinates` (the view
  box); `rkyv`, `serde_json` and `thiserror`.
- Used by:
  - `world_store`, which reads the regions;
  - the map engine (`legacy/map_engine`): its forest mass loader and lane, its buffer packer and
    chunk scheduler (the tree counts), its streaming host and its world loader (the regions);
  - the world export in `tools/map_assets/world_export_pipeline/src/`, which writes the
    regions archive and smooths the forest at `CANOPY_MASS_ISO`.
- Rules: chunk borders stitch without a seam and north is texture row 0
  (`stitch_shared_border_identity`, `y_flip_north_is_tex_row_zero`, `island_dims_pin` in
  `src/tests/density_tests.rs`); the density grid's texels sum to the exact tree count
  (`r3_texel_sum_equals_exact` in `src/tests/canopy_tests.rs`); the regions archive decodes to
  exactly the JSON regions, and refuses rows the JSON parser would drop and other schema versions
  (`everon_regions_archive_equals_the_json_regions`,
  `rows_the_json_parser_would_drop_are_refused_on_the_binary_route`,
  `wrong_schema_version_is_refused_even_though_the_bytes_validate` in
  `src/tests/regions_tests.rs`); world objects tier 4 (`cargo xtask verify crate-tiers`).
