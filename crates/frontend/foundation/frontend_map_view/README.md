# Frontend map view

The `frontend_map_view` crate: the shared seam every page uses to put a live terrain map on a
canvas. It sizes the canvas, creates the render engine, fits the camera to a terrain's world
bounds, runs the damage-driven frame pump, tracks resizes, turns pointer drags, wheel turns and
clicks into pans, zooms and picks in map metres, and samples ground heights at the terrain's native
2 m resolution. The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) builds its
canvas mount from these parts; a map picker mounts a whole view with one call.

## Contents

```text
crates/frontend/foundation/frontend_map_view/
├── Cargo.toml  the package: the camera, streaming model and elevation crates, the map and browser crates for wasm32, layout tier 8
└── src/        the camera fit, sizing, navigation arithmetic, heights, preferences, the error and the browser mount
```

## How it works

A map picker calls `mount::mount_map_view` with its container, canvas, terrain and preferences:
the mount sizes the canvas, reads the terrain manifest's `worldBounds`, fits the camera, creates
the engine, starts the pump, attaches the resize observer and the pointer listeners, and boots the
terrain. A missing manifest or an engine that does not start ends the mount with the crate's
`Error` before any listener is attached. The Mission Creator takes the parts one by one instead,
because its boot interleaves the mission document. The [source tree README](src/README.md) draws
the mount step by step and lists each module.

The crate is split by target. The camera fit, the canvas size arithmetic, the navigation
arithmetic, the ground heights, the picker's preferences and the error compile everywhere and are
tested natively. The modules that call the DOM, the render engine or the terrain boot
(`engine_mount`, `frame_pump`, `handles`, `mount`, `navigation`, `resize`) are gated to the
wasm32 build, where their crates live.

## Getting started

Run from the repository root:

```bash
cargo test -p frontend_map_view   # the camera fit, the canvas size, the navigation arithmetic, the heights, the preferences
```

## Configuration

None: no feature, no environment variable. A page URL carrying `force=webgl` makes
`engine_mount::force_webgl_from_location` choose the WebGL backend; terrain assets come from the
same-origin `/map-assets/<terrain>/`.

## Public surface

- `camera_fit` (`WorldBounds`, `ViewState`, `fit_view`), `device_size`, `navigation_math`,
  `terrain_height::TerrainHeights` and `terrain_preferences`, on every target.
- `Error` and `Result` (`error`): why a map view could not mount.
- In the wasm32 build: `engine_mount`, `frame_pump`, `handles::MapViewHandles`, `mount`
  (`MapViewMount`, `mount_map_view`), `navigation` (`MapClick`, `attach_navigation`) and
  `resize::observe_container_resize`.
- `prelude`: `Error`, `WorldBounds`, `ViewState` and `TerrainHeights`.

## Boundaries

- Depends on: `camera_math`, `map_streaming_model`, `terrain_elevation`, `serde`, `serde_json`,
  `thiserror`; `browser_platform`, `gpu_frame`, `map_renderer`, `map_streaming_host`, `web-sys`,
  `js-sys` and `wasm-bindgen` in the wasm32 build only.
- Used by: the single-page app (`apps/frontend`): the Mission Creator's canvas mount, boot tasks,
  input listeners and frame loop, and the mortar calculator's map picker and heights.
- Rules: the crate depends on no frontend crate (`cargo xtask ci verify-workspace-laws`); a module
  that calls the browser or a wasm32-only crate is gated on its `pub mod` line, and every other
  module compiles natively with its tests.

## Related documentation

- [Elevation model](/crates/terrain/terrain_elevation/README.md) — the full-resolution raster and
  the vector grid the heights come from.
- [Frontend documentation](/documentation/apps/frontend/README.md#shared-foundations) — the shared
  foundations among the routes, pages and workspaces of the app.
