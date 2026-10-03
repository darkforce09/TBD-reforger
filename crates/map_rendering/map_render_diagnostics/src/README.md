# Map render diagnostics source

The source of `map_render_diagnostics`: how the render engine measures itself in the browser —
byte-exact readback checks of its pipelines and calibration quads, the one-pixel readback of the
live scene, the frame benchmark and the stress pool. The engine statistics, `stats`, are the map
renderer's (`crates/map_rendering/map_renderer/src/engine_statistics.rs`). The
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) publishes the checks and the
benchmark to the editor gate.

## Contents

```text
crates/map_rendering/map_render_diagnostics/src/
├── benchmark/  `render_bench`, the stress-quad pool and the stress scene it fills
├── lib.rs      the crate root: module header and `mod` lines
├── prelude.rs  the functions the browser hooks publish, and the stress scene generator
└── readback/   byte-exact offscreen checks of the calibration quads and each pipeline, and one-pixel readback of the scene
```

## How it works

Every module but the stress scene compiles only for wasm32. Each diagnostic is a function that
takes `map_renderer`'s `RenderEngine` and reads it only through the views of
`map_renderer::diagnostic_accessors` (device, pipeline resources, scene, stress pool), measuring
the real device on whichever backend the browser gave it, WebGPU or WebGL2. A check draws into its
own offscreen target with its own camera and pipelines, reads pixels back and resolves a promise to
a JSON verdict.

```text
Mission Creator viewport bridge                 diagnostics function
  window.__selfChecks.calibration()        ──▶  readback: calibration_self_check
  window.__selfChecks.texture()            ──▶  readback: texture_self_check
  window.__selfChecks.<name>_self_check()  ──▶  readback: the other seven pipeline checks
  window.__selfChecks.readback_rgba(x, y)  ──▶  readback: readback_rgba
  window.__editorBench(n)                  ──▶  benchmark: render_bench
  window.__editorBench.seed_stress(n, s)   ──▶  benchmark: seed_stress (and clear_stress)
  window.__editorBench.compute_cull_*      ──▶  map_renderer: the compute-cull readings (engine methods)
  debug HUD, once a second                 ──▶  map_renderer: stats

editor gate, `selfcheck` smoke under WebGL2: calls both self-checks, fails unless both pass
```

The benchmark's clocks are `time_source`'s; the GPU frame timer is `gpu_device`'s, which the map
renderer holds.

## Public surface

- `readback::calibration::calibration_self_check`, `readback::texture::texture_self_check` and
  `benchmark::frame_benchmark::render_bench`: for the Mission Creator's viewport bridge and,
  through it, the editor gate.
- The other readback checks, `readback::scene::readback_rgba`, and
  `benchmark::stress_pool::{seed_stress, clear_stress}`: for the same browser hooks, under their own
  names, with no gate calling them.
- `benchmark::stress_scene::{stress_chunk, stress_chunk_into}`: the deterministic stress quads.
- `prelude`: all of the above.
- `RenderEngine::disable_frame_timing`, for the hosts that boot an engine, is the map renderer's
  (`crates/map_rendering/map_renderer/src/diagnostic_accessors.rs`); the Arsenal paper doll's
  self-check is `paper_doll_renderer`'s (`crates/paper_doll/paper_doll_renderer/`).

## Boundaries

- Depends on: `map_renderer` (the engine's diagnostic views, its clear colour and the packet's
  pipeline table), `gpu_frame` (the pipeline constructors, the batch and packet types, the packet
  encoder, the compute cull), `renderer_core` (packet binding ids), `symbology_layers_gpu` (the
  icon uniforms and the pooled lane test), `time_source` (the benchmark's clocks), `camera_math`
  (the check cameras), `map_draw_lanes::lane_roles` (lane ids), `map_coordinates` (the anchor),
  `render_primitives` (instance layouts, line vertices, text packing, the CPU cull oracle), and
  `wgpu`, `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys` and `web-sys`.
- Used by:
  - the Mission Creator's viewport bridge
    (`apps/frontend/src/workspaces/editor/bridge/viewport.rs`);
  - the editor gate's smokes in `tools/browser_testing/browser_gate_suites/`, which
    `cargo xtask mk leptos-gates` runs.
- Rules: a check never writes the engine's frame tables, camera uniform or batch list, so it can
  run beside the live render loop; expected pixels are exact bytes, except the ±1 the marquee's
  translucent blend is allowed.
