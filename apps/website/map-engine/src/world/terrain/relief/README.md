# Terrain relief

How the 2D map shows the shape of the ground, all computed from the elevation model: a hillshade
image, contour lines with each summit's ring drawn in a second colour, and the sea band, the
shallow-to-deep fills along and below the waterline.

## Contents

```text
apps/website/map-engine/src/world/terrain/relief/
├── contours.rs   contour levels, marched rings and segments, and the pick of each summit's ring
├── hillshade.rs  the hillshade image of the metres cache, at most 1024 pixels a side
├── host.rs       `DemVectors`: the vector grid, and the sea band and contour lanes for each zoom
├── mod.rs        the module tree
├── sea_band.rs   the sea band fills at four heights around the waterline, and their opacity by zoom
└── tests/        unit tests for the contours, the hillshade and the sea band
```

## How it works

`build_hillshade_image` runs once at boot, from `crate::streaming::host`: it keeps every nth
sample of the metres cache so that neither side passes `MAX_EDGE` (1024) pixels, shades each pixel
by Horn's gradient under a sun at azimuth 315° and altitude 45°, and returns an opaque grey RGBA
image whose rows run in the reverse order of the cache's. The host uploads it as the hillshade
texture layer.

`DemVectors` holds the vector grid, built once by `downsample_dem_grid` over a 12 800 m square
(`TERRAIN_M`, Everon's extent), and `sync(engine, zoom)` keeps two lanes current:

- Contours: the zoom's interval comes from `contour_interval_for_zoom` in `crate::overlay::lod`;
  an interval of 50 m or more marches a grid reduced once, 100 m or more twice
  (`contour_grid_reductions`). `contour_levels` lists the positive multiples of the interval up to
  the grid's highest point, and `contour_rings` marches each level into polylines, each a
  `ContourRing` (its level, whether it closes, its points, a closed ring not repeating its first
  point). `summit_ring_indices` picks every closed ring with no higher closed ring inside it, and
  `crate::world::mesh::compose_two_tone_contours` draws those in the summit colour. The lane is
  rebuilt only when the interval changes, and cleared while the zoom hides the `contour` class.
- Sea band: `build_sea_band_geometry` fills, for each of the `SEA_BAND_LEVELS` (5 m, 0 m, −2.5 m
  and −5 m, each with its colour), the cells at or below that height: one rectangle per run of
  cells wholly at or below it along a row, and a marching-squares polygon in each boundary cell,
  a saddle settled by the cell's mean. Rings close by repeating their first point.
  `sea_fill_alpha` gives the layer's opacity: 1 up to zoom 1, 0.6 up to 2, 0.3 up to 3 and none
  beyond; the host triangulates the fills (`compose_sea_mesh` in
  `crate::world::terrain::water::mesh`) when the opacity changes and clears the lane while the
  `sea` class is hidden.

`contour_segments` marches every level in one sweep into loose segments; only its tests call it.

## Boundaries

- Depends on: `crate::world::terrain::dem::grid` (the vector grid) and
  `crate::camera::math::shaping` (rounding); for the host, `crate::overlay::lod` (class gates and
  the contour interval), `crate::overlay::lanes` (lane ids), `crate::world::mesh` (the contour
  hairlines), `crate::world::terrain::water::mesh` (the sea mesh) and `crate::frame` (the engine
  handle).
- Used by: `crate::streaming::host`, which builds the hillshade at boot and owns the
  `DemVectors`; `crate::world::mesh`, which composes `ContourRing`s; and
  `crate::world::terrain::water::mesh`, which triangulates the `SeaBandGeometry`.
- Rules: `host.rs` compiles only for wasm32 with the `render` feature; flat ground shades one
  uniform grey (`flat_grid_is_uniform_cos_zenith` in `tests/hillshade_tests.rs`); each peak gets
  exactly one summit ring, its highest closed one, and an open chain never counts
  (`per_peak_selects_one_highest_closed_ring_each`,
  `ring_closure_distinguishes_closed_loop_from_open_chain`, `no_summit_rings_when_nothing_closes` in
  `tests/contours_tests.rs`); land above every sea level gets no fill and every fill ring is
  closed (`all_high_land_has_no_fill`, `rings_are_closed` in `tests/sea_band_tests.rs`).
