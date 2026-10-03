# Symbology instances and the slot GPU bridge

The browser-side bridge on `RenderEngine` that binds, patches and drags the icon lanes of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map entities
([slots](/documentation/glossary/n_to_z.md#slot), vehicles, comments, briefing markers, clusters)
on the GPU. The 20-byte icon instance packers live in `overlay_instances`; `symbols` is
re-exported here as the path `crate::camera::viewport` reads `cluster_mode` through. Compiled with
the `streaming` feature; the bridge and lane files only on `wasm32` with `render`.

## Contents

```text
legacy/map_engine/src/overlay/symbology/instances/
├── bridge_1.rs  `SlotGpuBridge`; atlas setup, slot binds, selection, drag and cluster entry points
├── bridge_2.rs  slot stats and clear; atlas upload, zoom uniform, lane rematerialise, drag overlay
├── bridge_3.rs  row patches, lane uploads, vehicle, marker and comment binds, icon pools
├── lanes.rs     `upload_icon_lane` for world trees, props and badges, and the icon uniform layout
└── mod.rs       the module tree and the `symbols` re-export of `overlay_instances::symbols`
```

## How it works

```text
Mission Creator bridge ──► RenderEngine (bridge_1..3)
  ensure_slot_atlas        unit_symbology::symbol_atlas widens the slot atlas, upload_slot_atlas
  slots_bind_symbology     overlay_instances::symbols::pack_slot_symbology ─► slot lane (or cluster discs)
  set_selection            overlay_instances::patches::symbology_row_patch per flipped row
  set_drag                 overlay_instances::drag::classify_drag_transition ─► overlay | delta | clear
  vehicles_bind_symbology, markers_bind, comments_bind_ids ─► their own icon lanes
```

Every instance is 20 bytes (`SLOT_ICON_STRIDE`): world position, size, yaw, glyph index and packed
RGBA, written by `render_primitives::text::pack`. While the camera
shows at most `SYMBOLOGY_MAX_M_PER_PX` (8 m per pixel), a slot draws as its role glyph, turned to
its heading and tinted by side, and a selected slot uses the selected cell block in
`SLOT_SELECTED_RGBA`; farther out every slot is a plain disc. More than `CLUSTER_SLOT_THRESHOLD`
(500) slots at zoom `ZOOM_CLUSTER_MAX` (−4) or below switch the lane to cluster discs sized by
count. A selection change patches only the rows that flipped, 12 bytes each at instance offset 8,
so a click never repacks the lane. A drag uploads one overlay of the dragged rows, hides them in
the base lane, then moves them with a shader uniform (`Delta`) until the drag ends. World icon
lanes (trees, props, building badges from the streamed world) arrive through `upload_icon_lane`,
which converts world positions to the scene anchor.

## Public surface

- `symbols`: the re-export of `overlay_instances::symbols`, for `crate::camera::viewport`'s
  `cluster_mode` call.
- `bridge_1::SlotGpuBridge`, `bridge_1::SlotAtlasGpu` and `lanes::ICON_UNIFORM_BYTES`, for
  `crate::frame`.
- The `#[wasm_bindgen]` methods on `RenderEngine` (`ensure_slot_atlas`, `slots_bind_symbology`,
  `set_selection`, `set_drag`, `vehicles_bind_symbology`, `vehicles_bind`, `markers_bind`,
  `comments_bind_ids`, `slot_stats_json`, `upload_icon_lane` and the rest), for the Mission
  Creator's canvas mount, document host, pointer gestures and viewport, and for
  `crate::streaming::loaders::world_loader`.

## Boundaries

- Depends on: `overlay_instances` (`symbols`, `drag`, `patches`; `symbols` re-exported),
  `unit_symbology` (`symbol_atlas`, `classification`), `map_draw_lanes` (`LaneRole`, `lane_id`,
  `px_to_m_at_zoom`), `spatial_indexes::point_indexes::cluster`, `crate::frame` (`RenderEngine`, bindings,
  draw batches, instance buffers, `create_glyph_atlas`), `map_coordinates::terrain_frames` (`ANCHOR`, `EVERON_BOUNDS`), `render_primitives`
  (`text::pack`, `draw::instances::ATLAS_GLYPH_COUNT`) and `wasm_bindgen`.
- Used by:
  - `crate::frame` (boot, encode, engine, lifecycle), `crate::camera::viewport` and
    `crate::streaming::loaders::world_loader`;
  - the Mission Creator in `apps/frontend/src/workspaces/editor/` (canvas mount boot tasks,
    document host, entity selection, attributes modal, armed placement, pointer gestures, select
    tool, viewport).
- Rules: a selection change patches rows and never repacks the lane, and the side tints stay three
  distinct colours with BLUFOR as the default (`selected_overrides_side_tint`,
  `side_tint_three_distinct`, `missing_side_defaults_blufor` in
  `crates/map_overlay/overlay_instances/src/tests/slot_instances/cases_1.rs`); the
  symbology degrades to dots past the stated scale
  (`symbology_degrades_to_dots_past_the_stated_m_per_px`); the bind paths are pinned by
  `legacy/map_engine/src/frame/tests/lane_bind_source_pins/symbology_bind_paths.rs`.

## Related documentation

- [Mission Creator feature inventory: performance at scale](/documentation/apps/frontend/workspaces/editor/feature_inventory/performance_at_scale.md) — the selection patches, drag overlay and clusters at scale.
