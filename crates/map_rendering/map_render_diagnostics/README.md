# Map render diagnostics

The `map_render_diagnostics` crate: how the map's render engine measures itself in the browser.
It holds the byte-exact offscreen readback checks of the engine's calibration quads and of each
pipeline, the one-pixel readback of the live scene, the frame benchmark and the stress-quad pool,
each a function over `map_renderer`'s `RenderEngine` that reads it only through the engine's
diagnostic views. The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
viewport bridge publishes them on `window.__selfChecks` and `window.__editorBench`, where the
editor gate calls them.

## Contents

```text
crates/map_rendering/map_render_diagnostics/
├── Cargo.toml  the package: `map_renderer`, `gpu_frame`, `renderer_core`, `symbology_layers_gpu`, `map_draw_lanes`, `camera_math`, `map_coordinates`, `render_primitives`, `time_source`, `bytemuck`; `wgpu`, `js-sys`, `wasm-bindgen`, `wasm-bindgen-futures` and `web-sys` on wasm32; layout tier 6, wasm32
└── src/        the readback checks, the scene readback, the frame benchmark, the stress pool and the stress scene with its tests
```

## How it works

```text
viewport bridge ──window.__selfChecks.<check>()──▶ readback check ──diagnostic views──▶ RenderEngine
                                                       │
                                                       └─ own target, camera, pipelines ─▶ copy ─▶ map ─▶ JSON verdict
viewport bridge ──window.__editorBench(n)──▶ render_bench ──encode_untimed_main_pass × n──▶ timings JSON
viewport bridge ──__editorBench.seed_stress(n, seed)──▶ stress pool ──stress pool view──▶ Stress lane batches
```

A check never touches the engine's frame tables, camera uniform or batch list, so it runs beside
the live render loop; only the stress pool writes the engine, and only through its stress pool
view. The source README details each module.

## Getting started

Run from the repository root:

```bash
cargo test -p map_render_diagnostics                                                        # the stress scene's byte cases
cargo clippy -p map_render_diagnostics --target wasm32-unknown-unknown --all-targets -- -D warnings  # the checks themselves
cargo xtask mk leptos-gates                                                                 # the editor gate's `selfcheck` smoke runs the checks in a browser
```

## Configuration

None: no feature and no environment variable. The checks are selected by the `wasm32` target.

## Public surface

- `readback::*::*_self_check` (calibration, texture, world building, sea band, road centreline,
  tree glyph, text, marquee, compute cull) and `readback::scene::readback_rgba`, each resolving a
  `js_sys::Promise` to its JSON report (WebAssembly).
- `benchmark::frame_benchmark::render_bench`, `benchmark::stress_pool::{seed_stress,
  clear_stress}` (WebAssembly) and `benchmark::stress_scene::{stress_chunk, stress_chunk_into}`.
- `prelude`: all of the above.

## Boundaries

- Depends on: `map_renderer` (the engine and its diagnostic views), `gpu_frame` (pipelines,
  packet, encoder, compute cull), `renderer_core` (binding ids), `symbology_layers_gpu`,
  `map_draw_lanes`, `camera_math`, `map_coordinates`, `render_primitives`, `time_source`,
  `bytemuck`; `wgpu`, `js-sys`, `wasm-bindgen`, `wasm-bindgen-futures` and `web-sys` in the
  WebAssembly build.
- Used by: the Mission Creator's viewport bridge
  (`crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/viewport.rs`).
- Rules: map rendering tier 6 (`cargo xtask verify crate-tiers`); a diagnostic names no engine
  field, only `map_renderer::diagnostic_accessors`.

## Related documentation

- [Map rendering overview](/documentation/crates/map_rendering/map_rendering_overview.md) — the
  rendering crates and the path from a mounted canvas to a drawn frame.
- [Map renderer](/crates/map_rendering/map_renderer/README.md) — the engine the diagnostics
  measure and its diagnostic views.
