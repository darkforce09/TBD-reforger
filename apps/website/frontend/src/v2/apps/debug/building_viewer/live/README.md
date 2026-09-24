# Building viewer browser wiring

The browser lifecycle of the building viewer: one function that boots the render engine on the
bench's canvas and wires every fetch, upload and listener of the live surface to the page's
signals. It compiles for `wasm32` only.

## Contents

```text
apps/website/frontend/src/v2/apps/debug/building_viewer/live/
└── wiring.rs  `wire`: fetches, engine boot, lane upload effects, pointer and resize listeners
```

## How it works

`BuildingViewerPage` calls `wire` once with its signals. `wire` reads the ray ends, `?force=webgl`
and `?doors=open` from the URL, then fetches the blueprint JSON, its `.bvh` occlusion sidecar and,
through `load_compound` in the parent file `live.rs`, the instances that assemble the compound
building; each failure lands in its own error signal, and the bench keeps whatever loaded. It
creates the `RenderEngine` on the canvas and starts the shared `RafPump`, and effects upload the
static lanes, the probe ray and the viewshed wash whenever their inputs change, fitting the
camera on the first blueprint. Pointer and wheel listeners on the canvas, with move and up on the
window, drag the observer and target, pan and zoom; an Alt+click moves the observer and lights
its viewshed, a click inside the footprint opens the floor rail, and a click on a door swings it.
Unmounting the page sets the `disposed` flag, which stops the frame pump; the listener closures
are released with `forget` and stay attached to the window and the canvas.

## Boundaries

- Depends on: the parent `apps/website/frontend/src/v2/apps/debug/building_viewer/live.rs`,
  through `use super::*` (the URL readers, `load_compound`, the lane uploaders, `sync_cam`, the
  page types, `geom` and `building_interior`); `website_map_engine::frame` (`RenderEngine`,
  `RafPump`) and the lane ids of `website_map_engine::overlay::lanes`; `gloo_net` for the fetches,
  `wasm_bindgen` and `web_sys`.
- Used by: `live.rs`, which re-exports `wire`, and
  `apps/website/frontend/src/v2/apps/debug/building_viewer/page.rs`, which calls it in the browser
  build.
- Rules: the engine handle lives only in the `Rc<RefCell<…>>` this function creates, never in a
  signal; the upload effects re-run once the engine is ready, so a blueprint that arrives before
  the engine still reaches the GPU; a missing sidecar turns line of sight, the wash and the mesh
  drawing off and says so, instead of failing the page. No test covers this browser-only file.

## Related documentation

- [Building viewer](/documentation_v2/website/frontend/apps/debug/building_viewer_page.md) — the
  bench's purpose and behaviour.
