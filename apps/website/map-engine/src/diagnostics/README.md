# Render diagnostics

How the render engine measures itself in the browser: byte-exact readback checks of its pipelines,
the calibration check, the frame benchmark and engine statistics, the frame clocks, and the console
macros the crate logs with. The [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)
publishes the checks and the benchmark to the editor gate.

## Contents

```text
apps/website/map-engine/src/diagnostics/
├── bench/     `render_bench`, the stress-quad pool, and `stats`, the engine's counters as JSON
├── mod.rs     the module tree
├── platform/  the crate's `log!`, `warn!` and `error!` console macros
├── probes/    `self_check`, the byte-exact calibration check
├── readback/  byte-exact offscreen checks of each pipeline, and one-pixel readback of the scene
└── timing/    the wall clocks and the GPU frame timer
```

## How it works

The module compiles only with the `render` feature (`apps/website/map-engine/src/lib.rs`), and
every file in it only for wasm32 as well. Most of it adds `#[wasm_bindgen]` methods to
`crate::frame::engine::RenderEngine` (and `doll_self_check` to the doll renderer's `DollEngine`),
each measuring the real device on whichever backend the browser gave it, WebGPU or WebGL2. A check
draws into its own offscreen target with its own camera and pipelines, reads pixels back and
resolves a promise to a JSON verdict.

```text
Mission Creator viewport bridge           engine method
  window.__selfChecks.calibration()  ──▶  probes: self_check
  window.__selfChecks.texture()      ──▶  readback: texture_self_check
  window.__editorBench(n)            ──▶  bench: render_bench
  debug HUD, once a second           ──▶  bench: stats

editor gate, `selfcheck` smoke under WebGL2: calls both self-checks, fails unless both pass
```

`timing/` and `platform/` serve the rest of the crate: `crate::frame` builds its GPU timer and times
each frame with them, and the satellite imagery and the streaming host log through the console
macros.

## Public surface

- `RenderEngine::self_check`, `texture_self_check`, `render_bench` and `stats`: for the Mission
  Creator's viewport bridge and, through it, the editor gate.
- `RenderEngine::poll` and `disable_frame_timing`: for the render loop in `crate::frame::pump` and
  for the hosts that boot an engine.
- The other readback checks, `readback_rgba`, `seed_stress` and `clear_stress`: exported to
  JavaScript, with no caller in the repository.
- Inside the crate: `timing::gpu::{GpuTimer, now_ms, perf_now_ms}` for `crate::frame`, and the
  console macros `platform::console::{log, warn, error}`.

## Boundaries

- Depends on: `crate::frame` (the engine, its pipelines, bindings, packet tables and compute cull),
  `crate::camera`, `crate::overlay::lanes` (lane ids), `crate::world::scene` (the anchor and the
  stress quads), `crate::doll` (the doll check), `website-graphics-engine` (`layout` and
  `draw::encode`), and `wgpu`, `wasm-bindgen`, `js-sys` and `web-sys`.
- Used by:
  - inside the crate: `crate::frame` (the timer, the clocks and `poll`),
    `crate::world::terrain::satellite` and `crate::streaming` (the console macros);
  - the Mission Creator's viewport bridge
    (`apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs`) and canvas boot, and the debug
    benches under `apps/website/frontend/src/v2/apps/debug/`;
  - the editor gate's smokes in `tools_v2/developer-tools/src/browser_testing/`, which
    `cargo xtask mk leptos-gates` runs.
- Rules: a check never writes the engine's frame tables, camera uniform or batch list, so it can
  run beside the live render loop; expected pixels are exact bytes, except the ±1 the marquee's
  translucent blend is allowed; the module names the renderer's frame vocabulary only through
  `crate::frame` (rule 3a of `cargo xtask verify engine-layers`).
