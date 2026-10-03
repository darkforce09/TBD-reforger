# Camera math

The `camera_math` crate: the cameras' f64 arithmetic. It holds the orthographic camera the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s tactical map draws with,
which reproduces deck.gl's orthographic viewport bit for bit at integer zooms, the orbit camera of
the [arsenal](/documentation/glossary/a_to_f.md#arsenal)'s doll preview, and the gl-matrix 4×4
routines both compose their matrices from. Nothing here touches a GPU or a browser binding.

## Contents

```text
crates/geometry/camera_math/
├── Cargo.toml  the package: `map_coordinates`, `serde_json` for the parity suite, layout tier 1
├── src/        the 4x4 routines, the scalar rules, the orthographic and orbit cameras, the prelude
└── tests/      integration suites: deck.gl parity over golden captures, seeded camera properties
```

## How it works

`ortho::state::OrthoCamera` is plain state (viewport size in CSS pixels, zoom as log2 pixels per
metre, its scale `2^zoom`, target in world metres, optional target bounds) that its owner moves
with `&mut` calls. Its matrices are composed in f64 in deck.gl's order, and the render uniform
`wgpu_clip_matrix` is cast to f32 last; `unproject_xy` inverts the pixel projection onto the
plane z = 0. The orbit camera fixes everything but the yaw and gives one view-projection for
picking and the same one behind a depth remap for rendering. `matrix4` keeps gl-matrix's
expression trees, which is what lets the parity suite assert zero ULP.

## Getting started

Run from the repository root:

```bash
cargo test -p camera_math   # the deck.gl parity suite and the seeded camera properties
```

## Public surface

- `ortho::state::{OrthoCamera, NEAR, FAR, MIN_ZOOM, MAX_ZOOM}`, with the camera's projection,
  unprojection and control methods; `OrthoCamera::with_scale_for_test`, hidden from the docs, for
  the parity suite alone.
- `orbit::projection::{view_proj_gl, view_proj_wgpu}`.
- `matrix4::{identity, multiply, translate_in_place, scale_in_place, ortho_no, perspective_no,
  look_at, invert, transform_vector, lerp2}`.
- `prelude`, which re-exports the cameras and their constants.

## Boundaries

- Depends on: `map_coordinates` (`rounding::round` for viewport sizes); `serde_json` in tests.
- Used by: the map engine (`legacy/map_engine`): its render engine, viewport entry points,
  editing tools, readback checks and doll import `ortho`, `orbit` and `matrix4`; and the
  single-page app (`apps/frontend`): the Mission Creator's toolbelt, the map view and the debug
  benches.
- Rules: the orthographic camera matches 300 deck.gl 9.3.5 captures, bit exact at integer zooms
  (`t1_integer_zoom_cases_bit_exact`) and with the golden scale injected
  (`t3_scale_injected_pipeline_bit_exact_all_cases`), within 2 ULP for matrices and 4 for
  projections end to end (`t4_end_to_end_bound_all_cases`); geometry tier 1, above
  `map_coordinates` (`cargo xtask verify crate-tiers`).

## Related documentation

- [Mission Creator feature inventory: map viewport and camera](/documentation/apps/frontend/workspaces/editor/feature_inventory/map_viewport_and_camera.md) — pan, zoom and the map view in the Mission Creator.
