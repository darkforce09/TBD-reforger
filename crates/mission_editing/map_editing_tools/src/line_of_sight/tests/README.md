# Line-of-sight tool tests

Unit tests of the line-of-sight tool: the ray capture and viewshed placement, the terrain and object
verdicts and their wording, the shot and profile projection, the object wash, the wash palette and
the texture payload, and the guard that a sight check never writes the mission document.

## Contents

```text
crates/mission_editing/map_editing_tools/src/line_of_sight/tests/
├── capture.rs           the two-click capture, the sub-mode toggle and the viewshed placement state
├── object_verdict.rs    goldens for the combined verdict language, the styling map and the block marker
├── object_wash.rs       goldens for the coarse-to-fine object pass over a synthetic viewshed
├── projection.rs        shot projection, panel keying and the elevation chart geometry
├── terrain_verdict.rs   the sight-line math, the occlusion rule and the readout formatters
├── viewshed_texture.rs  the 256-byte row padding and the upload-ready texture payload
└── wash_palette.rs      the viewshed raster encoder
```

## Boundaries

- Depends on: the line-of-sight modules (each file mounted beside its module with a `#[path]`
  attribute), `terrain_line_of_sight`, `world_line_of_sight`, the crate's `source_scrub`, and the
  dev-dependency `terrain_relief` for the contour colours the wash palette is checked against.
- Used by: `cargo test -p map_editing_tools`.
- Rules: every case builds its rasters and profiles in memory and needs no asset, browser or GPU.
