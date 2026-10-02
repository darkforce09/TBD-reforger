# Camera viewport

The render engine's camera entry points. The cameras themselves live in crates: the orthographic
camera of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s tactical map,
the orbit camera of the [arsenal](/documentation/glossary/a_to_f.md#arsenal)'s doll preview and
their matrix math in `crates/geometry/camera_math/`, the grid reference and JavaScript rounding in
`crates/geometry/map_coordinates/`; every caller imports them from those crates. This module, in
the browser build, gives the render engine the methods that move its camera.

## Contents

```text
legacy/map_engine/src/camera/
├── mod.rs       the module tree: `viewport`
└── viewport.rs  the render engine's resize, view, pan, zoom and camera-changed entry points
```

## How it works

`viewport.rs` is compiled only for wasm32 with the `render` feature. The render engine
(`crate::frame::engine::RenderEngine`) owns one `OrthoCamera`, and `viewport.rs` adds the
engine's JavaScript-facing methods that move it:

- `resize(css_w, css_h, dpr)` sizes the camera in CSS pixels and the surface at
  `round(css × dpr)` device pixels, and refuses a non-positive argument with `resize-nonpositive`;
- `set_view`, `pan`, `set_camera_bounds` and `zoom_at` forward to the camera's controls;
- `target_x`, `target_y` and `zoom` read the camera, and `visible_bounds` returns its visible world
  rectangle.

Every method that changes the camera or the surface marks the frame damaged, and each rendered
frame uploads `wgpu_clip_matrix` at the world anchor. `on_camera_changed`, which a host calls after
moving the camera, does nothing until the atlas of [slot](/documentation/glossary/n_to_z.md#slot) icons
is ready; then it refreshes the slot lanes' zoom uniform, asks
`crate::overlay::symbology::instances::symbols::cluster_mode` whether symbols cluster at the new
zoom, rebuilds the slot lane when that answer flips and no drag is live, and feeds the cluster
markers.

## Public surface

- `viewport`: the `RenderEngine` methods above, called by the Mission Creator's bridge and input
  handlers and by the debug benches.

## Boundaries

- Depends on: `crate::frame::engine::RenderEngine` (its `camera_math` orthographic camera,
  surface, device and damage), the slot and cluster methods of
  `crate::overlay::symbology::instances`, and `wasm-bindgen`.
- Used by: the Mission Creator's bridge and input handlers under
  `apps/frontend/src/v2/apps/editor/` and the debug benches under `apps/frontend/src/v2/apps/debug/`,
  through `RenderEngine`'s JavaScript-facing methods.
- Rules: only `viewport.rs` lives here, compiled for wasm32 with `render`; the cameras and the grid
  reference have one definition each, in `camera_math` and `map_coordinates`.

## Related documentation

- [Camera math](/crates/geometry/camera_math/README.md) — the orthographic and orbit cameras and
  their deck.gl parity.
- [Map coordinates](/crates/geometry/map_coordinates/README.md) — the grid reference and the
  rounding rule.
- [Mission Creator feature inventory: map viewport and camera](/documentation/apps/frontend/apps/editor/feature_inventory/map_viewport_and_camera.md) — pan, zoom and the map view in the Mission Creator.
