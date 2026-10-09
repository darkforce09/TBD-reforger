# Road export image modules

The submodules of `road_export_images.rs`, the `map road-images` lane: reading a Workbench road
export, stroking its six road layers onto RGBA canvases, and writing the layer and master PNGs.

## Contents

```text
tools/map_assets/map_raster_pipeline/src/road_export_images/
├── road_canvas.rs         `RoadCanvas`: straight-alpha RGBA pixels, the "over" blend, disc stamps, segment strokes, RGBA and RGB-over-background rows
├── road_image_outputs.rs  `draw_and_write_road_images`: draw order, world-to-pixel mapping, stroke radius, junction marks, the layer and master writes
└── road_layer_loading.rs  `load_road_export_meta` and `load_road_layers`: `roads_meta.json` and the six layer files read into typed segments and junctions
```

## How it works

```text
<roads dir>/roads_meta.json ─▶ load_road_export_meta ─▶ world size, junctions ─┐
<roads dir>/<stem>.json ×6  ─▶ load_road_layers      ─▶ RoadLayer ×6 ──────────┤
                                                                               ▼
          draw_and_write_road_images (ROAD_EXPORT_DRAW_ORDER: runways first, highways last)
            ├─ per layer that lists a segment: stroke onto the master and onto a fresh canvas
            │    └─▶ layer-<stem>.png (RGBA)
            ├─ --show-junctions: degree ≥ 3 discs on the master only
            └─▶ <terrain>-roads-transparent.png (RGBA), <terrain>-roads-dark.png (RGB over the dark background)
```

- `road_layer_loading` reads the JSON loosely, the way the Workbench export is written: a value is
  present only when truthy, so a `widthM` of 0 falls back to the class width and a `worldSizeM` of
  0 keeps the 12,800 m default; `null` and booleans count as 0 and 0/1 in arithmetic, while a
  missing value, a string, an array or an object is NaN and draws nothing. A point is
  `[x, y, z]` or `[x, z]`. A file that does not parse, or whose `totalLengthM` is truthy but not a
  number, is warned about and read as an empty layer. Each layer keeps the count of entries it
  lists, so a layer whose segments all have fewer than two points still writes its (empty) image.
- `road_image_outputs` maps world `(x, z)` to pixel `((x / world) × (size − 1), ((world − z) /
  world) × (size − 1))`, north up, and strokes each pair of consecutive points with a radius of
  `max(min width / 2, (width / 2) / (world / size))` in the class's `road_network` export style.
- `road_canvas` blends every disc visit of `grid_rasterization`'s `disc_coverage` with the "over"
  rule and rounds each result, a non-negative weighted mean, with ties toward +∞. The dark master
  blends each pixel over `ROAD_EXPORT_DARK_BACKGROUND_RGB` by its alpha.
- Every image streams row by row through `crate::image_operations::png_writing`.

## Boundaries

- Depends on: `road_network`'s export image styling (`road_export_image_style`,
  `ROAD_EXPORT_LAYER_FILES`, `ROAD_EXPORT_DRAW_ORDER`, the dark background and junction
  constants); `grid_rasterization`'s `disc_coverage` and `segment_stamp_centres`; `serde_json`;
  `crate::image_operations::png_writing`; `crate::error`.
- Used by: `tools/map_assets/map_raster_pipeline/src/road_export_images.rs`, whose `run` the
  `map road-images` command line calls.
- Rules: the decoded pixels of every image equal the reference images of the synthetic export
  (`fixture_export_draws_the_reference_images` in
  `tools/map_assets/map_raster_pipeline/src/tests/road_export_images_tests.rs`); the blend and row
  rules are pinned in `road_export_images_canvas_tests.rs` and the loose JSON rules in
  `road_export_images_layer_loading_tests.rs` beside it.
