# Spatial computation

The map engine's spatial folder: the browser upload that shows a finished viewshed raster as the
viewshed lane. The spatial computation itself lives in crates every caller imports directly:
[`spatial_indexes`](/crates/geometry/spatial_indexes/README.md) (the triangle BVH, the building
sidecar, the point grids and clusters) and the three line of sight crates under
`crates/line_of_sight/` (over the elevation model, inside one building, through the streamed
world).

## Contents

```text
legacy/map_engine/src/spatial/
├── los/    line of sight: the viewshed lane upload
└── mod.rs  the module tree
```

## How it works

Nothing here computes a spatial query. The
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s line-of-sight tool runs a
viewshed with `terrain_line_of_sight`, packs the raster into RGBA rows and hands them to
`los/terrain/overlay.rs`, which draws them as the `Viewshed` texture lane.

## Public surface

- `los`: see its README.

## Boundaries

- Depends on: `crate::frame` (`RenderEngine`), `crate::world` (the texture lane and the scene
  rectangle), `map_draw_lanes` (`LaneRole::Viewshed`), `wgpu` and `wasm-bindgen`, all for the
  viewshed upload only.
- Used by: the Mission Creator's input handlers and canvas mount in
  `apps/frontend/src/workspaces/editor/` and the debug building viewer in
  `apps/frontend/src/workspaces/debug/building_viewer/`, through `RenderEngine::viewshed_upload`
  and `viewshed_clear`.
- Rules: the module compiles only with the `world` feature (`legacy/map_engine/src/lib.rs`); the
  one file that reaches the browser, `los/terrain/overlay.rs`, compiles only for wasm32 with the
  `render` feature.
