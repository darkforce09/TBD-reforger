# Renderer core source

The source of `renderer_core`: the contracts between a renderer and its typed layers, the
renderer's statistics and the JSON writer it reports them with, the frame packet's binding ids,
and the crate root that declares them.

## Contents

```text
crates/graphics/renderer_core/src/
├── frame_hook.rs       `FrameHook` and `FrameHooks`: camera-changed and before-encode callbacks, in registration order
├── lane_sink.rs        `LaneSink`: a lane's batch upsert and removal, texture records, damage, layer context (WebAssembly)
├── layer_context.rs    `LayerContext`: the borrowed device, queue, surface format, packet tables and statistics (WebAssembly)
├── lib.rs              the crate root: module header and `mod` lines
├── packet_bindings.rs  the pipeline and bind-group ids a frame packet's tables are indexed by, and `tex_bind_id`
├── prelude.rs          the contracts, the statistics and the JSON writer
├── render_stats.rs     `RenderStats`: the last frame's CPU time, its moving average, the submitted flag, per-lane counts
├── stats_json.rs       `StatsJson`: one flat JSON object, field by field, in the caller's order
└── tests/              the native tests of the statistics, the JSON writer, the hooks and the binding ids
```

## How it works

`packet_bindings` fixes the order of a frame packet's tables: nine pipeline slots
(`PIPE_QUAD` … `PIPE_ICON_STORAGE32`) and five fixed bind-group slots (the camera, the glyph and
text atlases, and the movable-sprite atlas at rest and dragged), then one texture slot per lane at
`BIND_TEX_BASE + lane`. The renderer fills both tables in that order every frame; a layer stamps
the ids on the batches it builds.

`LaneSink<LaneTexture>` is the renderer's side of a lane. `upsert_lane_batch` inserts or replaces
the batch keyed by its `LaneId`, drops the lane's texture record and marks damage;
`remove_lane_batch` drops both and marks damage only when a batch went; `upsert_textured_lane_batch`
also keeps the renderer's `LaneTexture` record beside the batch until the lane is next upserted or
removed. `lane_batch` and `lane_batch_mut` read or adjust a batch in place, `mark_damage` forces
the next frame, `textured_lane_binding` names a lane's texture slot, and `layer_context` lends the
layer a `LayerContext`. The record type is a type parameter so the renderer can keep it private.

`FrameHooks<Renderer>` runs every registered `FrameHook` in registration order: `camera_changed`
after the renderer moves its camera, `before_encode` once per submitted frame. A hook overrides
only the callbacks it needs; `Renderer` is whatever the renderer lends its hooks.

`RenderStats` records each frame: `record_submitted_frame(cpu_ms)` stores the CPU time and folds it
into the moving average (`gpu_frame`'s `frame_ms_ema`, WebAssembly only), `record_skipped_frame`
clears the submitted flag; `set_lane_count` and `lane_count` keep one count per lane, zero for a
lane never counted. `StatsJson` writes the report: `text`, `count`, `flag`, `decimal` (exactly as
`format!("{:.places$}")` writes it) and `optional_decimal` (`null` when absent), with no
whitespace and with quotes, backslashes and control characters escaped.

## Public surface

- `frame_hook::{FrameHook, FrameHooks}`, `render_stats::RenderStats`, `stats_json::StatsJson`.
- `lane_sink::LaneSink` and `layer_context::LayerContext` (WebAssembly).
- `packet_bindings`: `PIPE_QUAD`, `PIPE_TEXTURED`, `PIPE_DENSITY`, `PIPE_LINE`,
  `PIPE_ORIENTED_QUAD`, `PIPE_POLYGON`, `PIPE_ICON`, `PIPE_TEXT`, `PIPE_ICON_STORAGE32`,
  `PIPELINE_SLOTS`, `BIND_CAMERA`, `BIND_GLYPH_ATLAS`, `BIND_TEXT_ATLAS`,
  `BIND_MOVABLE_SPRITE_ATLAS`, `BIND_MOVABLE_SPRITE_ATLAS_DRAGGED`, `BIND_TEX_BASE`,
  `tex_bind_id`.

## Boundaries

- Depends on: `gpu_frame::frame::DrawBatch` and `gpu_frame::frame::present::frame_ms_ema`;
  `render_primitives::frame::ids`; `wgpu` in the WebAssembly build.
- Used by: `map_renderer`, `symbology_layers_gpu`, `world_layers_gpu` and
  `map_render_diagnostics` (`crates/map_rendering/`).
- Rules: no name or document here names a thing in the world being drawn; `lane_sink.rs` and
  `layer_context.rs` compile for `wasm32` only, every other module on every target.
