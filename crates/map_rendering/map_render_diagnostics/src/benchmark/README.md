# Render benchmark

The render engine's benchmark readouts: `render_bench` times a run of offscreen frames, and the
stress-quad pool loads the map with synthetic instances. The engine's statistics report, `stats`,
is the map renderer's (`crates/map_rendering/map_renderer/src/engine_statistics.rs`).

## Contents

```text
crates/map_rendering/map_render_diagnostics/src/benchmark/
├── frame_benchmark.rs  `render_bench` over n offscreen frames of the live scene
├── mod.rs              the module tree
├── stress_pool.rs      the stress-quad pool: `seed_stress` and `clear_stress`
└── stress_scene.rs     `stress_chunk` and `stress_chunk_into`: deterministic quads over the Everon square
```

## How it works

`frame_benchmark.rs` and `stress_pool.rs` compile only for wasm32; `stress_scene.rs` is plain
arithmetic over `render_primitives`' `QuadInstance` and compiles natively, where its byte tests
run. The three functions take the engine and reach it only through
`map_renderer::diagnostic_accessors`. `render_bench(engine, n)` clamps n to 1–20 000, draws the
engine's current batch list n times into an offscreen target the size of the surface through
`RenderEngine::encode_untimed_main_pass`, and times each frame's CPU encode and its submit with
`time_source::monotonic_ms`; it then waits up to 3 s for the queue to drain and resolves to JSON
with `n`, `submit_wall_ms`, `total_wall_ms`, `cpu_avg_ms`, `cpu_p95_ms`, `cpu_max_ms`,
`submit_avg_ms`, `fps_equiv` and `drained`.

`seed_stress(engine, n, seed)` fills the `Stress` lane with n deterministic quads from
`stress_scene.rs`'s `stress_chunk_into`, in chunks of `render_primitives`' `CHUNK_CAPACITY`, and
records the generation and upload times, read on `time_source::BrowserClock`, through the engine's
stress pool view. `clear_stress(engine)`, which `seed_stress` calls first, keeps the last batch of
the list as the calibration batch and destroys every other batch's buffers, every textured lane's
texture and the lane pool.

## Boundaries

- Depends on: `map_renderer` (the engine's diagnostic views: device, scene and stress pool, and
  `encode_untimed_main_pass`), `gpu_frame::frame` (the batch and payload types),
  `renderer_core::packet_bindings` (`PIPE_QUAD`), `symbology_layers_gpu::slot_symbology`
  (`is_pooled_icon_lane`), `map_draw_lanes::lane_roles` (lane ids),
  `map_coordinates::terrain_frames` (the anchor), `time_source` (the clocks),
  `crate::readback::scene` (the async sleep), and `render_primitives::draw::instances`
  (`QuadInstance`, `CHUNK_CAPACITY`).
- Used by: the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s viewport
  bridge (`crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/viewport.rs`), which publishes
  `render_bench` as `window.__editorBench(n)` and `seed_stress` and `clear_stress` as its
  properties; the editor gate's smoke harness in
  `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests.rs`, which calls
  `window.__editorBench` when it exists. No gate calls `seed_stress` or `clear_stress`.
- Rules: the benchmark draws through the same main pass as a live frame, so it measures the real
  frame path.
