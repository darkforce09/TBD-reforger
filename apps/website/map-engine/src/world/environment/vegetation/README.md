# Vegetation: forest mass, tree counts and land cover

How the 2D map shows vegetation: the forest mass, a density texture of trees across the whole
island with its outline traced from the same grid; the tree counts that decide between drawing
each tree and letting the forest mass stand in; and the land-cover regions (forest, field, water
body) of the terrain.

## Contents

```text
apps/website/map-engine/src/world/environment/vegetation/
├── buffers.rs  the engine's forest density texture lane and its fill and outline settings
├── canopy.rs   tree and vegetation counts over chunks, and the switch from tree glyphs to the fill
├── density.rs  the island density grid: stitching per-chunk tree counts, packing the texture
├── loader.rs   `ForestMassHost`: fetches the density bins, uploads the forest fill and outline
├── mass.rs     marching squares over the density grid: outlines, fill polygons, the alpha ladder
├── mod.rs      the module tree
├── regions.rs  land-cover regions from `forest-regions.json.gz` or its archive
└── tests/      unit tests for the counts, the density grid, the marching squares and the regions
```

## How it works

`ForestMassHost` builds the forest mass once per terrain. It fetches the 625 density bins
`objects/density/{cx}_{cy}.bin` from the terrain's asset folder (`assets_v2/terrains/everon/` for
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

## Boundaries

- Depends on: `crate::io::density` (the density bin decoder) and `crate::io::archives` (the
  regions archive); `crate::world::environment::classify` (class codes); `crate::overlay::lod`
  (the instance budget and the zoom gates); `crate::streaming` (chunks, fetch, boot progress, the
  statistics bridge); and, for the browser files, `crate::world::mesh` (hairline composition),
  `crate::world::scene`, `crate::frame`, `crate::overlay::lanes` and `wgpu`.
- Used by:
  - `crate::streaming`: the host owns the `ForestMassHost`, the buffer packer takes the tree
    counts, and the world loader and the store read the regions;
  - `crate::world::mesh`, whose forest compose takes the marching squares' `ForestMassGeometry`;
  - the world export in `tools_v2/developer-tools/src/world_export_pipeline/`, which writes the
    regions archive and smooths the forest at `CANOPY_MASS_ISO`.
- Rules: chunk borders stitch without a seam and north is texture row 0
  (`stitch_shared_border_identity`, `y_flip_north_is_tex_row_zero`, `island_dims_pin` in
  `tests/density_tests.rs`); the density grid's texels sum to the exact tree count
  (`r3_texel_sum_equals_exact` in `tests/canopy_tests.rs`); the regions archive decodes to exactly
  the JSON regions, and refuses rows the JSON parser would drop and other schema versions
  (`everon_regions_archive_equals_the_json_regions`,
  `rows_the_json_parser_would_drop_are_refused_on_the_binary_route`,
  `wrong_schema_version_is_refused_even_though_the_bytes_validate` in `tests/regions_tests.rs`);
  `buffers.rs` and `loader.rs` compile only for wasm32 with the `render` feature, and `canopy.rs`
  and `regions.rs` only with `streaming`.
