# Vegetation source

The source of `vegetation`: the regions, the forest mass, the tree counts, the density grid, the
error and the crate root that declares them.

## Contents

```text
crates/world_objects/vegetation/src/
├── canopy.rs   tree and vegetation counts over chunks, and the switch from tree glyphs to the fill
├── density.rs  the island density grid: stitching per-chunk tree counts, packing the texture
├── error.rs    `Error` and `Result`: the archive's `BinaryError` behind one type
├── lib.rs      the crate root: module header, `mod` lines and re-exports
├── mass.rs     marching squares over the density grid: outlines, fill polygons, the alpha ladder
├── prelude.rs  the names most readers import
├── regions.rs  land-cover regions from `forest-regions.json.gz` or its archive
└── tests/      unit tests for the counts, the density grid, the marching squares and the regions
```

## How it works

`density` and `mass` work on the island grid of tree-count corners; `canopy` counts the chunk rows
a view draws; `regions` reads and writes the land-cover polygons. None of them depends on another.

## Boundaries

- Depends on: `world_file_formats`, `world_chunks`, `prefab_catalog`, `map_draw_lanes`,
  `map_coordinates`, `rkyv`, `serde_json` and `thiserror`.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: no module here fetches, uploads or touches a browser API.
