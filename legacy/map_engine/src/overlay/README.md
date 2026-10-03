# Map overlay

The map engine's half of the map overlay: the lane visibility and tint preferences and the
symbology's GPU bridges. The lane roles, zoom gates, labels, symbology and instance packers live in
the map overlay crates under `crates/map_overlay/`, which every caller imports directly. It decides
identity and legibility; the GPU buffers and draw calls belong to `crate::frame` and
`graphics_engine`, which see only an opaque lane key.

## Contents

```text
legacy/map_engine/src/overlay/
├── lanes_prefs.rs  layer visibility, texture lane opacity, clear colour, the 1 km grid
├── mod.rs          the module tree: `lanes_prefs` and `symbology`
└── symbology/      the glyph atlas upload and the slot instance GPU bridges
```

## How it works

```text
LaneRole ──lane_order──► rank 0..47 ──lane_id──► frame::LaneId(2·rank [+1 for Calibration])
   ▲                                                   │ graphics_engine sorts on it
   │ role_id (vector uploads)                          ▼
   │ tex_role_id (basemap, hillshade)          batches drawn bottom to top
browser API u32
```

The lane roles are `map_draw_lanes::lane_roles` and the zoom gates `map_draw_lanes::zoom_gates`.
`lane_order` ranks every `LaneRole` from the basemap up to the [mission](/documentation/glossary/g_to_m.md#mission) lanes and the marquee;
`lane_id` doubles the rank so `Stress` and `Calibration`, which share rank 0, get distinct keys.
`lanes_prefs.rs` adds `RenderEngine` methods on `wasm32` with `render`: `set_world_layer_visible`
maps a layer name (`roads`, `forest`, `contours`, `sea`, `airfield`, `heights`, `townLabels`,
`roadNames`) to its lanes, `set_lane_opacity` re-tints a texture lane in place, and `set_grid`
builds the 1 km grid.

## Public surface

- `lanes_prefs`: `set_world_layer_visible`, `set_lane_opacity`, `set_grid` and `set_clear_color`
  on `RenderEngine`, for `crate::streaming::host` and `crate::streaming::loaders::world_loader`,
  and the debug apps' clear colour.
- `symbology`: see its README.

## Boundaries

- Depends on: `map_draw_lanes`, `crate::frame`
  (`RenderEngine`, draw batches and payloads, bindings), `render_primitives` (`LaneId`, the grid
  and line vertices), `map_coordinates::terrain_frames::ANCHOR` for the grid, `graphics_engine`
  (`draw::lines`) and `wasm_bindgen`. The module needs the crate's `world` feature, and `lanes_prefs` needs
  `wasm32` with `render`.
- Used by: `crate::streaming` (the host preferences and the world loader's viewport) and
  `crate::camera` (the `symbols` re-export under `symbology::instances`); the
  [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) and the debug apps under
  `apps/frontend/src/workspaces/` reach the GPU bridges through `RenderEngine`.
- Rules: the lane order, the wire ids and the zoom gates are pinned in `map_draw_lanes`
  (`crates/map_overlay/map_draw_lanes/src/tests/`); the renderer names no lane (rule 2 of
  `cargo xtask verify engine-layers`).
