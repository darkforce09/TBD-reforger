# Map renderer

The `map_renderer` crate: `RenderEngine`, the renderer of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map canvas and of the debug
benches' map views. It creates the canvas's GPU through `gpu_device`'s `GpuContext`, builds the
shader, layouts and pipelines, owns the orthographic camera and the persistent batch list, holds
the symbology and world typed layers as fields and lends them its lanes, writes the vector, marquee
and label lanes, encodes one frame packet per damaged frame, reports its statistics as JSON and
serves the streaming host as its asset sink.

## Contents

```text
crates/map_rendering/map_renderer/
├── Cargo.toml  the package: `renderer_core`, `gpu_frame`, `gpu_device`, `render_primitives`, `camera_math`, `map_coordinates`, `map_draw_lanes`, `symbology_layers_gpu`, `world_layers_gpu`, `map_streaming_model`, `time_source`; `wgpu` and `web-sys` on wasm32; layout tier 5, wasm32
└── src/        the engine, its boot, frame path, viewport, statistics, asset sink, lane sinks, typed layer doors, upload belts, error type and their tests
```

## How it works

```text
host ──RenderEngine::create(canvas, force_webgl)──▶ GpuContext ("map-engine-render", timestamps when offered)
     └─ RafPump on EngineHandle ──▶ render(): damage gate ─▶ acquire ─▶ encode_main_pass ─▶ submit ─▶ poll
streaming host and loaders ──MapAssetSink──▶ asset sink ─▶ engine belts / symbology layers / world layers
Mission Creator ──with_symbology / with_terrain_line_of_sight_overlay / belts / viewport──▶ engine
every lane write ──renderer_core::LaneSink──▶ sorted batch list ─▶ damage marked ─▶ next render
```

A frame is drawn only when something marked it damaged: a lane write, a camera move, a resize or
`mark_dirty`. The packet borrows the engine's batch list and refills its pipeline and bind-group
tables in place, so nothing the size of the scene is rebuilt per frame. The typed layers never see
the engine: each call lends them a lane sink borrowed apart from the rest of the engine. The
source README details each module.

## Getting started

Run from the repository root:

```bash
cargo test -p map_renderer                                                        # the pins, the statistics JSON, the calibration bytes, the surface size, the errors
cargo clippy -p map_renderer --target wasm32-unknown-unknown --all-targets -- -D warnings  # the engine itself
cargo xtask verify crate-anatomy                                                  # lib.rs, prelude, error, README and manifest shape
```

## Configuration

None: no feature and no environment variable. The engine is selected by the `wasm32` target.

## Public surface

- `RenderEngine`, `EngineHandle` and `CLEAR_COLOR` (at the root and in `engine`; WebAssembly):
  `create`, `render`, `poll`, `resize`, the viewport, damage and clear-colour methods, the upload
  belts, `with_symbology`, `with_terrain_line_of_sight_overlay`, `stats`, `slot_stats_json`, the
  compute-cull switches and counters, the text atlas and the texture limits.
- `encode::pipeline_table` and `diagnostic_accessors` (the device, pipeline resources, scene and
  stress pool views, `encode_untimed_main_pass`, `disable_frame_timing`) for the render
  diagnostics (WebAssembly).
- `RenderEngine` implements `renderer_core::lane_sink::LaneSink`, `gpu_frame`'s `FrameTarget` and
  `map_streaming_model`'s `MapViewport` and `MapAssetSink`.
- `Error`, `Result`; `prelude`: the engine, its slot and the error.

## Boundaries

- Depends on: `renderer_core` (lane sink, layer context, frame hooks, render statistics, binding
  ids), `gpu_frame` (frame vocabulary, uploads, encoder, compute cull, pipelines, frame pump),
  `gpu_device` (the GPU context and the frame timer), `render_primitives`, `camera_math`,
  `map_coordinates`, `map_draw_lanes`, `symbology_layers_gpu`, `world_layers_gpu`,
  `map_streaming_model` (the asset sink contract), `time_source` (the frame clock), `bytemuck`,
  `thiserror`; `wgpu` and `web-sys` in the WebAssembly build.
- Used by: the Mission Creator, the map views and the debug benches of the frontend
  (`crates/frontend/foundation/frontend_map_view/src/`, `crates/frontend/workspaces/`), and the render
  diagnostics in `crates/map_rendering/map_render_diagnostics/src/`.
- Rules: map rendering tier 5 (`cargo xtask verify crate-tiers`); the frame path is damage-driven
  and refills in place; a typed layer is lent a lane sink, never
  the engine.

## Related documentation

- [Map rendering overview](/documentation/crates/map_rendering/map_rendering_overview.md) — the
  rendering crates and the path from a mounted canvas to a drawn frame.
- [GPU rendering overview](/documentation/crates/graphics/gpu_rendering_overview.md) — the GPU
  device and frame crates the engine draws with.
- [Map streaming](/documentation/crates/streaming/map_streaming.md) — the host and loaders that
  write through the asset sink.
