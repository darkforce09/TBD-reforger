# Symbology instances and the slot GPU bridge

Packs the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s map entities
([slots](/documentation_v2/glossary/n_to_z.md#slot), vehicles, comments, briefing markers, clusters) into
20-byte icon instances, and the browser-side bridge on `RenderEngine` that binds, patches and drags
those lanes on the GPU. Compiled with the `streaming` feature; the bridge and lane files only on
`wasm32` with `render`.

## Contents

```text
apps/website/map-engine/src/overlay/symbology/instances/
├── bridge_1.rs  `SlotGpuBridge`; atlas setup, slot binds, selection, drag and cluster entry points
├── bridge_2.rs  slot stats and clear; atlas upload, zoom uniform, lane rematerialise, drag overlay
├── bridge_3.rs  row patches, lane uploads, vehicle, marker and comment binds, icon pools
├── drag.rs      `DragGpuPhase`, the drag transition rule and the drag overlay and preview packers
├── lanes.rs     `upload_icon_lane` for world trees, props and badges, and the icon uniform layout
├── mod.rs       the module tree; re-exports the graphics engine's `text::pack` as `packing`
├── patches.rs   12-byte row patches for selection and hiding, and the selection-only pack
├── slots/       one re-export surface of this folder's vocabulary, and the tests that pin it
└── symbols.rs   instance sizes and colours, the cluster gate, slot, vehicle and comment packers
```

## How it works

```text
Mission Creator bridge ──► RenderEngine (bridge_1..3)
  ensure_slot_atlas        atlas::raster widens the slot atlas, upload_slot_atlas
  slots_bind_symbology     symbols::pack_slot_symbology ─► slot lane (or cluster discs)
  set_selection            patches::symbology_row_patch per flipped row (O(changed rows))
  set_drag                 drag::classify_drag_transition ─► overlay upload | delta uniform | clear
  vehicles_bind_symbology, markers_bind, comments_bind_ids ─► their own icon lanes
```

Every instance is 20 bytes (`SLOT_ICON_STRIDE`): world position, size, yaw, glyph index and packed
RGBA, written by the graphics engine's `text::pack` (re-exported as `packing`). While the camera
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

- `symbols`: the instance constants, `cluster_mode`, `px_to_m_at_zoom` and the packers, for
  `crate::camera::viewport`, `crate::overlay::symbology::markers` and `crate::frame`.
- `drag::pack_vehicle_drag_preview`, for the Mission Creator's select tool
  (`apps/website/frontend/src/v2/apps/editor/input/tools/select_tool.rs`).
- `bridge_1::SlotGpuBridge`, `bridge_1::SlotAtlasGpu` and `lanes::ICON_UNIFORM_BYTES`, for
  `crate::frame`.
- The `#[wasm_bindgen]` methods on `RenderEngine` (`ensure_slot_atlas`, `slots_bind_symbology`,
  `set_selection`, `set_drag`, `vehicles_bind_symbology`, `vehicles_bind`, `markers_bind`,
  `comments_bind_ids`, `slot_stats_json`, `upload_icon_lane` and the rest), for the Mission
  Creator's canvas mount, document host, pointer gestures and viewport, and for
  `crate::streaming::loaders::world_loader`.

## Boundaries

- Depends on: `crate::frame` (`RenderEngine`, bindings, draw batches, instance buffers),
  `crate::overlay::lanes` (`LaneRole`, `lane_id`), `crate::overlay::symbology::atlas` and
  `crate::overlay::symbology::roles`, `crate::world::scene` (`ANCHOR`, `EVERON_BOUNDS`),
  `website_graphics_engine` (`text::pack`, `layout::ATLAS_GLYPH_COUNT`) and `wasm_bindgen`.
- Used by:
  - `crate::frame` (boot, encode, engine, lifecycle), `crate::camera::viewport`,
    `crate::overlay::symbology::markers` and `crate::streaming::loaders::world_loader`;
  - the Mission Creator in `apps/website/frontend/src/v2/apps/editor/` (canvas mount boot tasks,
    document host, entity selection, attributes modal, armed placement, pointer gestures, select
    tool, viewport).
- Rules: a selection change patches rows and never repacks the lane, and the side tints stay three
  distinct colours with BLUFOR as the default (`selected_overrides_side_tint`,
  `side_tint_three_distinct`, `missing_side_defaults_blufor` in `slots/tests/cases_1.rs`); the
  symbology degrades to dots past the stated scale
  (`symbology_degrades_to_dots_past_the_stated_m_per_px`); the bind paths are pinned by
  `apps/website/map-engine/src/overlay/tests/tests/draw_order_t808_symbology_bind_paths.rs`.

## Related documentation

- [Mission Creator feature inventory: performance at scale](/documentation_v2/website/frontend/apps/editor/feature_inventory/performance_at_scale.md) — the selection patches, drag overlay and clusters at scale.
