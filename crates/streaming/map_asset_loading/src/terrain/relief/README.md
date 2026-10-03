# Terrain relief host

The browser side of the terrain relief: the `DemVectors` that keeps the contour and sea lanes current
for each zoom. The relief itself (hillshade, contour rings with summit picks, sea band) is the
[`terrain_relief`](/crates/terrain/terrain_relief/README.md) crate, which every caller imports
directly.

## Contents

```text
crates/streaming/map_asset_loading/src/terrain/relief/
├── dem_vectors.rs  `DemVectors`: the vector grid, and the sea band and contour lanes for each zoom
└── mod.rs          the module tree: `dem_vectors`
```

## How it works

`DemVectors` holds the vector grid, built once by `downsample_dem_grid` over a 12 800 m square
(`TERRAIN_M`, Everon's extent), and `sync(engine, zoom)` keeps two lanes current:

- Contours: the zoom's interval comes from `contour_interval_for_zoom` in `map_draw_lanes::zoom_gates`;
  `terrain_relief::contours` reduces the grid, lists the levels, marches the rings and picks the
  summit rings, and `crate::mesh_composition::compose_two_tone_contours` draws those in the summit
  colour. The lane is rebuilt only when the interval changes, and cleared while the zoom hides
  the `contour` class.
- Sea band: `terrain_relief::sea_band` builds the fills once and gives the layer's opacity by
  zoom; the host triangulates them (`water_bodies::mesh::compose_sea_mesh`) when the opacity
  changes and clears the lane while the `sea` class is hidden.

## Boundaries

- Depends on: `terrain_relief`, `terrain_elevation` (the vector grid), `map_draw_lanes` (class
  gates, the contour interval and lane ids), `crate::mesh_composition` (the contour hairlines), `water_bodies` (the sea mesh) and
  the asset sink of `map_streaming_model`, held as the crate's `BrowserAssetSinkHandle`.
- Used by: the map host of `map_streaming_host`, which builds the hillshade at boot and owns
  the `DemVectors`.
- Rules: `dem_vectors.rs` compiles only for wasm32.
