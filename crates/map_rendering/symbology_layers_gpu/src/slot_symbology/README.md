# Slot symbology layer

The typed GPU layer of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
[slot](/documentation/glossary/n_to_z.md#slot) symbology: `SlotSymbologyGpu` owns the slot atlas,
the slot bridge's columns, selection and drag, and the pooled buffers the slot, drag, cluster,
preview, vehicle and comment lanes draw from; `SlotSymbology` is that state at work, borrowed with
the renderer parts a bind writes through. Compiled only on `wasm32`.

## Contents

```text
crates/map_rendering/symbology_layers_gpu/src/slot_symbology/
├── atlas.rs          `ensure_slot_atlas`, the slot atlas upload and the zoom uniform sync
├── clusters.rs       `camera_changed` and the cluster marker lane
├── drag.rs           `set_drag`: the drag overlay, its uniform offset and its release
├── mission_lanes.rs  vehicle, marker (with captions), comment and placement preview lanes
├── mod.rs            the module tree and the public re-exports
├── pooled_lanes.rs   the pooled sprite lane uploads, the compute cull hand-off, `is_pooled_icon_lane`
├── slot_lane.rs      `slots_bind_symbology`, the slot lane repack and the selection patches
├── state.rs          `SlotSymbologyGpu`, the slot bridge and the slot atlas handles
└── view.rs           `SlotSymbology`, its shared helpers, and `TextAtlasSupply`
```

## How it works

```text
RenderEngine::with_symbology(|symbology| …)
  └─ SlotSymbologyGpu::at_work(icon cull, lanes, camera, text atlas, uniform counter)
       └─ SlotSymbology ── binds ──► pooled_lanes ──► LaneSink<Infallible> (the renderer's lanes)
                                              └──► IconCullGpu (when the compute cull is in use)
```

The renderer builds a `SlotSymbology` for each call: the layer's own state plus a
`&mut dyn LaneSink<Infallible>` (sprite lanes carry no texture record), the compute cull, the
camera whose zoom gates the symbology and clusters, the shared text atlas the marker captions
sample (`TextAtlasSupply`), and the renderer's uniform byte counter. Every lane write ends in
`pooled_lanes.rs`: the packed icons go into the lane's pooled buffer and the lane's sprite batch
is upserted or patched in place, or, with the compute cull in use, the icons go to the cull and
the batch is removed. The renderer's camera frame hook calls `camera_changed` after every camera
move.

## Public surface

- `SlotSymbologyGpu`: `new`, `at_work`, `atlas_ready`, `has_atlas`, `atlas_bytes`,
  `atlas_bind_group`, `dragged_atlas_bind_group`, `clear_lane_pool`, `append_slot_stats`, for
  `map_renderer`'s render engine (`crates/map_rendering/map_renderer/src/`) and
  `map_render_diagnostics`' stress bench.
- `SlotSymbology`: `ensure_slot_atlas` (refused with `Error::SlotAtlasPixelLength`),
  `slots_bind_symbology`, `set_selection`, `set_drag`, `vehicles_bind`, `vehicles_bind_symbology`,
  `markers_bind`, `comments_bind`, `comments_bind_ids`, `set_place_preview`,
  `clear_place_preview`, `camera_changed`, for the Mission Creator and the mortar map picker
  through `RenderEngine::with_symbology`, and the renderer's camera hook.
- `TextAtlasSupply`, implemented by the renderer's text atlas slot; `is_pooled_icon_lane`, for the
  stress bench's lane teardown.

## Boundaries

- Depends on: `overlay_instances`, `unit_symbology`, `map_draw_lanes`, `spatial_indexes`,
  `renderer_core`, `gpu_frame`, `gpu_device`, `camera_math`, `map_coordinates`,
  `render_primitives`, and the crate's `icon_uniforms`, `icon_cull_gpu` and `Error`.
- Used by: `map_renderer`'s render engine (`crates/map_rendering/map_renderer/src/`: the typed
  layers' accessor and camera hook, encode, cull, statistics, boot) and `map_render_diagnostics`.
- Rules: the bind bodies are pinned by
  `crates/map_rendering/map_renderer/src/tests/lane_bind_source_pins/symbology_bind_paths.rs` (which reads
  `atlas.rs`, `slot_lane.rs` and `mission_lanes.rs`) and `comments_bind_skips_pick_bridge.rs`
  (which reads `mission_lanes.rs` and needs `comments_bind` before `comments_bind_ids` there).
