# Line of sight

The map engine's line of sight folder: the browser upload that shows a viewshed raster as the
viewshed lane. The three line of sight layers are crates their users import directly:
[`terrain_line_of_sight`](/crates/line_of_sight/terrain_line_of_sight/README.md) over the
elevation model, [`interior_line_of_sight`](/crates/line_of_sight/interior_line_of_sight/README.md)
inside one building and [`world_line_of_sight`](/crates/line_of_sight/world_line_of_sight/README.md)
through the streamed world.

## Contents

```text
legacy/map_engine/src/spatial/los/
├── mod.rs     the module tree
└── terrain/   the viewshed lane upload
```

## Boundaries

- Depends on: for `terrain/overlay.rs`, `crate::frame::engine` (`RenderEngine`),
  `map_draw_lanes::lane_roles` (`LaneRole::Viewshed`), `crate::world::terrain::satellite`
  (`TexLane`), `wgpu` and `wasm-bindgen`.
- Used by: the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s input
  handlers and canvas mount and the debug building viewer in `apps/frontend/src/workspaces/`.
- Rules: `terrain/overlay.rs` compiles only for wasm32 with the `render` feature.
