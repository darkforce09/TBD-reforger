# Map overlay

What the map draws and in what order: the 48 named lanes and their paint order, the zoom gates
that decide what is legible at a scale, the lane visibility and tint preferences, and the
symbology drawn in the lanes. It decides identity and legibility; the GPU buffers and draw calls
belong to `crate::frame` and `website-graphics-engine`, which see only an opaque lane key.

## Contents

```text
apps/website/map-engine/src/overlay/
├── lanes.rs        `LaneRole`: 48 lanes, paint order, renderer key, two wire-id sets
├── lanes_prefs.rs  layer visibility, texture lane opacity, clear colour, the 1 km grid
├── lod.rs          zoom gates per world render class, the instance budget and the contour interval
├── mod.rs          the module tree
├── symbology/      role, vehicle and marker glyphs, atlases, instances, labels and squad links
└── tests/          unit tests for the lane order, the wire ids, the zoom gates and the bind paths
```

## How it works

```text
LaneRole ──lane_order──► rank 0..47 ──lane_id──► frame::LaneId(2·rank [+1 for Calibration])
   ▲                                                   │ website-graphics-engine sorts on it
   │ role_id (vector uploads)                          ▼
   │ tex_role_id (basemap, hillshade)          batches drawn bottom to top
browser API u32
```

`lane_order` ranks every `LaneRole` from the basemap up: satellite, sea, hillshade, landcover and
contours, roads, buildings, fences and forest, world glyphs and labels, the building interior
lanes, the viewshed and the interior probe, the 1 km grid, then the
[mission](/documentation_v2/glossary.md#mission) lanes (zones, markers,
comments, connections, squad links, vehicles, [slots](/documentation_v2/glossary.md#slot), place
preview, drag, clusters) and the marquee
on top. `lane_id` doubles the rank so `Stress` and `Calibration`, which share rank 0, get distinct
keys. The browser speaks two disjoint `u32` namespaces: `role_id` for the vector-lane uploads and
`tex_role_id` (`BASEMAP` 0, `HILLSHADE` 1) for the texture lanes, so id 0 is `Sea` in one and
`Satellite` in the other. `ALL_LANES` lists every variant in declaration order, which differs from
paint order for `Grid` (declared after `Clusters`, ranked 35).

`lod::class_visible` answers whether a world render class is drawn, and pickable, at a zoom (trees
from zoom 0, building footprints from −2.5, badges from 1, props from 3, forest fill below 0, and so
on), `INSTANCE_BUDGET` (150 000) caps drawn world instances, and `contour_interval_for_zoom` picks a
5 m to 80 m interval that keeps contour spacing near `TARGET_SPACING_PX` on screen. `lanes_prefs.rs`
adds `RenderEngine` methods on `wasm32` with `render`: `set_world_layer_visible` maps a layer
name (`roads`, `forest`, `contours`, `sea`, `airfield`, `heights`, `townLabels`, `roadNames`) to
its lanes, `set_lane_opacity` re-tints a texture lane in place, and `set_grid` builds the 1 km grid.

## Public surface

- `lanes`: `LaneRole`, `lane_order`, `lane_id`, `ALL_LANES`, `role_id`, `tex_role_id` and their
  `u32` conversions, for `crate::frame`, `crate::streaming`, `crate::world`, `crate::spatial`,
  `crate::diagnostics`, the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
  document host and tools, and the debug apps in
  `apps/website/frontend/src/v2/apps/debug/`.
- `lod`: the zoom gates, `REF_ZOOM`, `INSTANCE_BUDGET` and the contour interval, for
  `crate::streaming` (bridge, buffers, scheduler) and `crate::world` (vegetation, relief).
- `lanes_prefs`: `set_world_layer_visible`, `set_lane_opacity`, `set_grid` and `set_clear_color`
  on `RenderEngine`, for `crate::streaming::host` and `crate::streaming::loaders::world_loader`,
  and the debug apps' clear colour.
- `symbology`: see its README.

## Boundaries

- Depends on: `crate::frame` (`RenderEngine`, `LaneId`, draw batches and payloads, bindings),
  `crate::world::scene::ANCHOR` for the grid, `website_graphics_engine` (`draw::lines`,
  `text::scale::REF_ZOOM`) and `wasm_bindgen`. The module needs the crate's `world` feature;
  `lanes` and `lod` also need `streaming`, `lanes_prefs` needs `wasm32` with `render`, and
  `symbology` gates its own files.
- Used by: `crate::frame`, `crate::streaming`, `crate::world`, `crate::spatial::los::terrain`,
  `crate::diagnostics`, `crate::editing`, `crate::camera`; the Mission Creator and the debug apps
  under `apps/website/frontend/src/v2/apps/`; the architecture tests in
  `tools_v2/xtask/src/verifications/architecture/tests/`.
- Rules: the `role_id` and `tex_role_id` wire ids never change (`wire_ids_are_pinned`,
  `tex_wire_ids_are_pinned` in `tests/draw_order.rs`); `ALL_LANES` covers every variant
  (`all_lanes_covers_every_variant`); each lane keeps its neighbours in paint order (the `*_sit_*`
  tests there, such as `grid_sits_between_world_glyphs_and_mission_lanes`); the renderer names no
  lane (rule 2 of `cargo xtask verify engine-layers`). `BUILDING_FOOTPRINT_MIN_ZOOM` (−2.5) is also
  written as `BUILDING_MIN_ZOOM` in `apps/website/map-engine/src/streaming/buffers/revision.rs`,
  so a change here must change both.
