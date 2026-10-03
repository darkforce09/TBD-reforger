# Line of sight over terrain

The browser upload that shows a finished viewshed raster as the viewshed lane. The elevation
profile, the viewshed and the sliced viewshed job are the
[`terrain_line_of_sight`](/crates/line_of_sight/terrain_line_of_sight/README.md) crate.

## Contents

```text
legacy/map_engine/src/spatial/los/terrain/
├── mod.rs      the module tree: `overlay`
└── overlay.rs  `viewshed_upload` and `viewshed_clear`, which show a raster as the viewshed lane
```

## How it works

In the browser, the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
line-of-sight tool packs a finished raster into RGBA rows, and `viewshed_upload` shows them as the
`Viewshed` texture lane over the raster's world rectangle; its rows must be at least four bytes a
texel and 256-byte aligned. `viewshed_clear` removes the lane.

## Boundaries

- Depends on: `crate::frame::engine` (`RenderEngine`, `BasemapMode`), `map_draw_lanes::lane_roles`
  (`LaneRole::Viewshed`), `crate::world::terrain::satellite::textures` (`TexLane`),
  `crate::world::scene` (`world_rect_rel`), `wgpu` and `wasm-bindgen`.
- Used by: the Mission Creator's input handlers
  (`apps/frontend/src/workspaces/editor/input/`) and canvas mount, and the debug building viewer
  (`apps/frontend/src/workspaces/debug/building_viewer/`), which upload and clear the lane.
- Rules: `overlay.rs` compiles only for wasm32 with the `render` feature.
