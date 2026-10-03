# Renderer core

The `renderer_core` crate: the map-agnostic contracts a renderer and the typed layers that fill it
meet at. `LaneSink` is what a layer needs from the renderer to own a lane (upsert and remove its
draw batch, keep a textured lane's record, mark damage, name its texture slot); `LayerContext`
lends a layer the renderer's device, queue, surface format, packet tables and statistics;
`FrameHook` and `FrameHooks` are the per-frame callbacks the renderer runs before it encodes;
`RenderStats` keeps the frame and lane counters and `StatsJson` writes the flat JSON object a
renderer reports them as; `packet_bindings` numbers the frame packet's pipeline and bind-group
slots. It knows no map concept: a lane is an opaque `LaneId`.

## Contents

```text
crates/graphics/renderer_core/
├── Cargo.toml  the package: `gpu_frame`, `render_primitives`; `wgpu` on wasm32; layout tier 2, wasm32
└── src/        the contracts, the statistics, the JSON writer, the binding ids and their tests
```

## How it works

```text
typed layer ──&mut dyn LaneSink<_>──▶ renderer: upsert_lane_batch / remove_lane_batch
     │                                   │  (persistent batch list, damage, texture records)
     └─ layer_context() ─▶ LayerContext: device, queue, surface format, packet tables, RenderStats
renderer: camera moved ─▶ FrameHooks::camera_changed ─▶ every hook, in registration order
          each frame    ─▶ FrameHooks::before_encode  ─▶ encode ─▶ RenderStats::record_*
stats() ─▶ StatsJson: keys in the caller's order ─▶ one flat JSON object
```

A renderer implements `LaneSink` over its own batch list and keeps one `RenderStats` and one
`FrameHooks`; a layer maps its own lane names onto `LaneId`, stamps the ids of
`packet_bindings` on its batches and writes through the sink. The source README details each
module.

## Getting started

Run from the repository root:

```bash
cargo test -p renderer_core                                                    # statistics, JSON writer, hooks, binding ids
cargo clippy -p renderer_core --target wasm32-unknown-unknown --all-targets -- -D warnings  # the lane sink and layer context
cargo xtask verify crate-anatomy                                               # lib.rs, prelude, README and manifest shape
```

## Configuration

None: no feature and no environment variable. The lane sink and the layer context are selected
by the `wasm32` target.

## Public surface

- `lane_sink::LaneSink` and `layer_context::LayerContext` (WebAssembly).
- `frame_hook::{FrameHook, FrameHooks}`.
- `render_stats::RenderStats`; `stats_json::StatsJson`.
- `packet_bindings`: the `PIPE_*` pipeline ids, `PIPELINE_SLOTS`, the `BIND_*` bind-group ids,
  `BIND_TEX_BASE` and `tex_bind_id`.
- `prelude`: the contracts, the statistics and the JSON writer.

## Boundaries

- Depends on: `gpu_frame` (`DrawBatch`, the frame-cost moving average), `render_primitives`
  (`LaneId`, `PipelineId`, `BindGroupId`); `wgpu` in the WebAssembly build.
- Used by: `map_renderer`, whose render engine implements `LaneSink`, owns a `RenderStats` and
  reports `stats()` through `StatsJson`; `symbology_layers_gpu` and `world_layers_gpu`, which
  receive a `LayerContext`, write through `LaneSink` and stamp the `packet_bindings` ids on their
  batches; and `map_render_diagnostics`.
- Rules: graphics tier 2 (`cargo xtask verify crate-tiers`); declares no map noun; a lane is an
  opaque `LaneId`, never a caller's lane role; a module that names a GPU type is `wasm32` only, so
  `cargo test -p renderer_core` runs natively.

## Related documentation

- [GPU rendering overview](/documentation/crates/graphics/gpu_rendering_overview.md) — the
  graphics crates and one frame across them.
