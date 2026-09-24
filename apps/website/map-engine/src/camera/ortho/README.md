# Orthographic map camera

The top-down camera of the tactical map in the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) and the debug benches:
`OrthoCamera`, its matrices, its pan and zoom controls, and the conversion between screen pixels
and world metres. It reproduces deck.gl's orthographic viewport, bit for bit at integer zooms.

## Contents

```text
apps/website/map-engine/src/camera/ortho/
├── camera/         re-exports `OrthoCamera` and its constants; nothing imports this path
├── controllers.rs  target bounds, the clamped view setter, drag-pan and cursor-anchored zoom
├── mod.rs          the module tree
├── projection.rs   view, projection, pixel and render matrices, and world-to-pixel projection
├── state.rs        `OrthoCamera`: size, zoom, scale, target, bounds; the planes and the zoom band
└── unproject.rs    pixel-to-world unprojection and the visible world rectangle
```

## How it works

`OrthoCamera` holds the viewport size in CSS pixels, the zoom (log2 of pixels per metre), its scale
`2^zoom`, the target in world metres and optional target bounds. World x runs east and y north;
screen pixels run from the top left with y down. `new` and `resize` round the size as deck.gl's
`makeViewport` does and apply no clamp, because the parity goldens hold unclamped states.

`projection.rs` composes the matrices the way deck.gl's `Viewport` does: the view scales a look-at
from (0, 0, 1) by the scale and translates by the negated target; the projection is `ortho_no` over
the half-extents of the viewport with the planes `NEAR` 0.1 and `FAR` 1000; `view_projection` keeps
deck.gl's identity product for its sign of zero; `pixel_projection` puts the viewport matrix in
front, so `project` maps a world point to top-left pixels. `wgpu_clip_matrix(anchor_x, anchor_y)`
is the render uniform: the `Z01` depth remap, the view-projection and a translation by the anchor,
composed in f64 and cast to f32 last, since instance geometry is stored relative to
`crate::world::scene::ANCHOR`. `unproject.rs` inverts the pixel projection and interpolates
between depths 0 and 1 onto the world plane z = 0; it answers `[NaN, NaN]` when the matrix is
singular. `visible_world_rect` takes the component-wise minimum and maximum of the four corners.

`controllers.rs` holds the only clamps: `set_view` limits the zoom to `MIN_ZOOM` −6 to `MAX_ZOOM`
6 and the target to the bounds; `pan` moves the target so the content follows the cursor;
`zoom_at` changes the zoom within the same band and keeps the world point under the cursor where
it was on screen. `pan` and `zoom_at` keep the target within the bounds too.

## Public surface

- `state::OrthoCamera` with `new`, `resize` and the accessors `target_x`, `target_y`, `zoom`,
  `scale` and `size_px`, and `state::{NEAR, FAR, MIN_ZOOM, MAX_ZOOM}`: the camera of
  `crate::frame`, `crate::editing` and `crate::diagnostics`, and of the Mission Creator's
  toolbelt and the debug benches.
- `controllers`: `set_bounds`, `set_view`, `pan` and `zoom_at`.
- `projection`: `view_matrix`, `projection_matrix`, `view_projection`, `pixel_projection`, `project`
  and `wgpu_clip_matrix`.
- `unproject`: `pixel_unprojection`, `unproject_xy` and `visible_world_rect`.
- `OrthoCamera::with_scale_for_test`, hidden from the docs: a constructor that takes the scale as
  given, for `apps/website/map-engine/tests/deckgl_ortho_parity.rs` alone.

## Boundaries

- Depends on: `crate::camera::math` (`glmat4` and `dimensions`).
- Used by:
  - inside the crate: `crate::frame` (the render engine's camera), `crate::camera::viewport`,
    `crate::editing` (picking and the selection tools) and `crate::diagnostics` (the probe runner
    and the readback checks);
  - the Mission Creator's toolbelt and document helpers
    (`apps/website/frontend/src/v2/apps/editor/ui/docks/toolbelt.rs`,
    `apps/website/frontend/src/v2/apps/editor/mission_editor/document_helpers.rs`) and the debug
    benches under `apps/website/frontend/src/v2/apps/debug/`, which read `MAX_ZOOM`;
  - the integration suites `apps/website/map-engine/tests/camera_props.rs` and
    `apps/website/map-engine/tests/deckgl_ortho_parity.rs`.
- Rules: the camera matches deck.gl's orthographic viewport over 300 golden cases in
  `apps/website/map-engine/tests/fixtures/deckgl_ortho_goldens.json`, captured from deck.gl 9.3.5:
  bit exact at integer zooms (`t1_integer_zoom_cases_bit_exact`) and with the golden scale
  injected (`t3_scale_injected_pipeline_bit_exact_all_cases`), and within 2 ULP for matrices and
  4 for projections end to end (`t4_end_to_end_bound_all_cases`); round trips, pan, cursor-anchored
  zoom with its clamp, the visible rectangle and the bounds clamp hold over seeded random cameras
  (`camera_props.rs`).
