# Calibration readback check

The render engine's calibration check: it draws the two calibration quads offscreen with the quad
pipeline and compares seven pixels byte for byte, which proves the camera, the pipeline and
north-up orientation on whichever backend the browser gave the engine.

## Contents

```text
apps/website/map-engine/src/diagnostics/probes/
├── mod.rs     the module tree
└── runner.rs  `RenderEngine::self_check`: seven byte-exact pixel probes over the calibration quads
```

## How it works

`runner.rs` compiles only for wasm32 with the `render` feature. `self_check` builds an 800×600
`Rgba8Unorm` target, a camera centred on (6400, 6400) at zoom 0 (one pixel per metre) with its own
uniform, and a fresh quad pipeline, then draws the calibration instances the engine uploaded at boot
(`crate::world::scene::calibration_instances`). It copies the target into a buffer with rows padded
to 3328 bytes and polls the mapping every 4 ms, giving up after 2000 polls, about 8 s. The probes
read the centre of the green quad and two pixels 2 px inside its corners, the clear colour
(51, 68, 85) 2 px outside it, red inside the small quad in the north-east, and green at the same
offset to the south, which fails if the map is drawn upside down. The promise resolves to
`{"backend", "probes": [{px, py, expect, got, pass, label}], "pass"}` and rejects with
`probe-map-timeout: …` or `probe-map-failed`.

## Boundaries

- Depends on: `crate::frame` (the engine's device, queue, shader module, layouts, unit-quad and
  calibration buffers, `CLEAR_COLOR` and the quad pipeline constructor), `crate::camera::ortho`
  and `crate::world::scene::ANCHOR`.
- Used by: the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
  `window.__selfChecks.calibration`, published by
  `register_self_checks` in `apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs`; the
  editor gate's `selfcheck` smoke
  (`tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/editor_boot_scenarios.rs`)
  calls it under WebGL2, and `cargo xtask mk leptos-gates` runs that smoke.
- Rules: the check uses its own target, uniform and pipeline and leaves the engine's frame state
  alone, so it can run beside the live render loop; the expected bytes are exact, which holds
  because `CLEAR_COLOR` converts to unorm8 with no rounding ambiguity (the margin note on it in
  `apps/website/map-engine/src/frame/engine.rs`).
