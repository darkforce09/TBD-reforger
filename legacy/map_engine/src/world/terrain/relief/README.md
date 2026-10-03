# Terrain relief host

The map engine's side of the terrain relief: the host that keeps the contour and sea lanes current
for each zoom. The relief itself (hillshade, contour rings with summit picks, sea band) is the
[`terrain_relief`](/crates/terrain/terrain_relief/README.md) crate, which every caller imports
directly.

## Contents

```text
legacy/map_engine/src/world/terrain/relief/
├── host.rs  `DemVectors`: the vector grid, and the sea band and contour lanes for each zoom
└── mod.rs   the module tree: `host`
```

## How it works

`DemVectors` holds the vector grid, built once by `downsample_dem_grid` over a 12 800 m square
(`TERRAIN_M`, Everon's extent), and `sync(engine, zoom)` keeps two lanes current:

- Contours: the zoom's interval comes from `contour_interval_for_zoom` in `map_draw_lanes::zoom_gates`;
  `terrain_relief::contours` reduces the grid, lists the levels, marches the rings and picks the
  summit rings, and `crate::world::mesh::compose_two_tone_contours` draws those in the summit
  colour. The lane is rebuilt only when the interval changes, and cleared while the zoom hides
  the `contour` class.
- Sea band: `terrain_relief::sea_band` builds the fills once and gives the layer's opacity by
  zoom; the host triangulates them (`water_bodies::mesh::compose_sea_mesh`) when the opacity
  changes and clears the lane while the `sea` class is hidden.

## Boundaries

- Depends on: `terrain_relief`, `terrain_elevation` (the vector grid), `map_draw_lanes` (class
  gates, the contour interval and lane ids), `crate::world::mesh` (the contour hairlines), `water_bodies` (the sea mesh) and
  `crate::frame` (the engine handle).
- Used by: `crate::streaming::host`, which builds the hillshade at boot and owns the
  `DemVectors`.
- Rules: `host.rs` compiles only for wasm32 with the `render` feature.
