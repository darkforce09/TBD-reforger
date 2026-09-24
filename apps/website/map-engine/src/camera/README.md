# Cameras

The map engine's cameras: the orthographic camera the tactical map of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) draws with, the orbit camera of
the [arsenal](/documentation_v2/glossary.md#arsenal)'s doll preview, the matrix math both are built
from, and the grid reference printed on the map's edges. In the browser build it also gives the
render engine the entry points that move its camera.

## Contents

```text
apps/website/map-engine/src/camera/
├── grid_reference.rs  the 1 km grid spacing and the three-digit grid reference of a coordinate
├── math/              f64 matrix and scalar helpers that reproduce gl-matrix and deck.gl arithmetic
├── mod.rs             the module tree
├── orbit/             the doll preview's perspective camera, turned by yaw alone
├── ortho/             the tactical map's orthographic camera: matrices, controls, unprojection
└── viewport.rs        the render engine's resize, view, pan, zoom and camera-changed entry points
```

## How it works

`ortho::OrthoCamera` is plain f64 state and arithmetic that reproduces deck.gl's orthographic
viewport; `orbit/` builds the doll's perspective matrices from a yaw; both compose their matrices
with `math/`. Neither touches a GPU, so the module compiles in every build of the crate, the
[API](/documentation_v2/glossary.md#api)'s included.

The one exception is `viewport.rs`, compiled only for wasm32 with the `render` feature. The render
engine (`crate::frame::engine::RenderEngine`) owns one `OrthoCamera`, and `viewport.rs` adds the
engine's JavaScript-facing methods that move it:

- `resize(css_w, css_h, dpr)` sizes the camera in CSS pixels and the surface at
  `round(css × dpr)` device pixels, and refuses a non-positive argument with `resize-nonpositive`;
- `set_view`, `pan`, `set_camera_bounds` and `zoom_at` forward to the camera's controls;
- `target_x`, `target_y` and `zoom` read the camera, and `visible_bounds` returns its visible world
  rectangle.

Every method that changes the camera or the surface marks the frame damaged, and each rendered
frame uploads `wgpu_clip_matrix` at the world anchor. `on_camera_changed`, which a host calls after
moving the camera, does nothing until the atlas of [slot](/documentation_v2/glossary.md#slot) icons
is ready; then it refreshes the slot lanes' zoom uniform, asks
`crate::overlay::symbology::instances::symbols::cluster_mode` whether symbols cluster at the new
zoom, rebuilds the slot lane when that answer flips and no drag is live, and feeds the cluster
markers.

`grid_reference.rs` fixes the grid at `GRID_STEP_M`, 1000 m. `grid_ref_3digit` formats a world
coordinate as its hundreds of metres modulo 1000, zero-padded (6400 m reads `064`), and answers
`000` for a negative or non-finite input; `grid_lines_in_range` lists the grid lines inside a
span, so an edge label always sits on a drawn line.

## Public surface

- `ortho::state::OrthoCamera`, its constants and its control, projection and unprojection methods:
  the camera of `crate::frame`, `crate::editing` and `crate::diagnostics`, the Mission Creator's
  toolbelt and the debug benches.
- `grid_reference::{GRID_STEP_M, grid_ref_3digit, grid_lines_in_range}`: re-exported by the
  toolbelt's `apps/website/frontend/src/v2/apps/editor/ui/docks/toolbelt/grid_reference.rs` and
  `scale_math.rs`, and used by `crate::editing::commands::selection_digest`.
- `orbit::projection::{view_proj_gl, view_proj_wgpu}`: for `crate::doll`.
- `viewport`: the `RenderEngine` methods above, called by the Mission Creator's bridge and input
  handlers and by the debug benches.
- `math::glmat4`: public for the crate's own modules; nothing outside the crate imports it.

## Boundaries

- Depends on: nothing outside the folder, except `viewport.rs`, which extends
  `crate::frame::engine::RenderEngine` (its camera, surface, device and damage), calls the slot and
  cluster methods of `crate::overlay::symbology::instances`, and uses `wasm-bindgen`.
- Used by:
  - inside the crate: `crate::frame`, `crate::editing` (picking, the selection tools and the
    selection digest), `crate::doll`, `crate::diagnostics`, and `crate::world::terrain`, which
    rounds with `math::shaping::round`;
  - the Mission Creator's bridge, input handlers and toolbelt under
    `apps/website/frontend/src/v2/apps/editor/`, and the debug benches under
    `apps/website/frontend/src/v2/apps/debug/`;
  - the integration suites `apps/website/map-engine/tests/camera_props.rs` and
    `apps/website/map-engine/tests/deckgl_ortho_parity.rs`.
- Rules:
  - `ortho/` matches deck.gl's orthographic viewport: bit exact at integer zooms and within a few
    ULP elsewhere (the parity suite named in its README);
  - the grid reference has one convention: the Mission Creator's edge labels and its clipboard
    exporters take it from here (`grid_formatter_arma_3digit_with_wrap` and
    `labels_match_grid_lines` in
    `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/toolbelt/furniture_geometry.rs`,
    `the_exporter_grid_ref_is_the_map_furnitures_own_label_text` in
    `apps/website/frontend/src/v2/apps/editor/shell/tests/exporter_grid_reference.rs`);
  - only `viewport.rs` names the render engine or the overlay, so the rest of the module stays
    free of the `render` feature and of any browser binding.
