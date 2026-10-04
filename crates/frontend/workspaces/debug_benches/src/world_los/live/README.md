# World line-of-sight bench mount

The browser mount of the world line-of-sight bench: one function that reads the run from the URL,
boots the render engine on the bench's canvas, streams the object catalogue in around the chosen
point, and wires the pointer and the wheel to the probe and the camera. It compiles for `wasm32`
only.

## Contents

```text
crates/frontend/workspaces/debug_benches/src/world_los/live/
└── mount.rs  `mount`: URL run parameters, engine boot, catalogue load, lane upload, probe input
```

## How it works

`WorldLosPage` calls `mount` once with its `Signals`. `mount` reads `x`, `y`, `r` (clamped to 20
to 600 m), `eye`, `a`, `b` and `force` from the URL, then runs two tasks side by side: it creates
the `RenderEngine` on the canvas and starts the shared `RafPump`, and it calls `load` in the parent
file `live.rs`, which streams the catalogue window in and reports its progress on the status line.
When both are up, an effect builds the plan lanes with `scene_of` and `build_bench_lanes`, uploads
them and writes the stats line; when the URL gave no ray ends, A and B start 40 m either side of
the centre at eye height. A second effect probes again whenever A or B moves. On the canvas a drag
pans, a click without movement places A and then B in turn, and the wheel zooms.

## Boundaries

- Depends on: the parent `crates/frontend/workspaces/debug_benches/src/world_los/live.rs`, through
  `use super::*` (`Signals`, the URL readers, `load`, `scene_of`, `upload_lanes`, `probe`,
  `with_engine`, the bench defaults), and through it `build_bench_lanes` from
  `crates/frontend/workspaces/debug_benches/src/world_los_scene.rs` and `screen_to_world` from the
  building viewer's `geom`; `map_renderer` (`RenderEngine`) and `gpu_frame` (`RafPump`); `js_sys`,
  `wasm_bindgen` and `web_sys`.
- Used by: `live.rs`, which re-exports `mount`, and
  `crates/frontend/workspaces/debug_benches/src/world_los.rs`, whose `WorldLosPage` calls it in the
  browser build.
- Rules: the engine handle and the loaded bench live only in the `Rc<RefCell<…>>` cells this
  function creates; the lanes upload only once both the engine and the catalogue are ready; a
  wheel or pan listener reads the camera under a `with_engine` borrow and writes the `cam` signal
  only after that borrow ends. The
  mount registers no cleanup: nothing sets `disposed`, so the frame pump and the listener closures,
  released with `forget`, outlive the page. No test covers this browser-only file.
