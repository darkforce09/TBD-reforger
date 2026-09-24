# Orbit camera

The perspective camera of the doll preview in the [arsenal](/documentation_v2/glossary.md#arsenal):
it circles the mannequin at a fixed distance and height and turns by yaw alone. Pitch, distance
and pan are constants, with no control that changes them.

## Contents

```text
apps/website/map-engine/src/camera/orbit/
├── camera.rs      the orbit constants and the view matrix for a yaw
├── mod.rs         the module tree
└── projection.rs  the view-projections: `view_proj_gl` for picking, `view_proj_wgpu` to render
```

## How it works

`camera.rs` fixes the vertical field of view at 0.6109 rad, the planes at 0.1 and 100, the orbit
centre at (0, 1.02, 0), the distance at 3.3 and the eye height at 1.45. `view(yaw)` puts the eye at
(sin(yaw)·3.3, 1.45, cos(yaw)·3.3) and looks at the centre with +y up. `view_proj_gl` multiplies the
perspective by that view in GL clip space (depth in [-1, 1]), with an aspect of width over height,
or 1 when the height is not positive; the doll's picking inverts it. `view_proj_wgpu` puts the
`Z01` depth remap in front, composes in f64 and casts to f32 last, for the render uniform; each
instance's model matrix multiplies in the shader.

## Boundaries

- Depends on: `crate::camera::math::glmat4` (`look_at`, `perspective_no`, `multiply`).
- Used by: `crate::doll`, whose renderer, picking and scene model use both projections (the scene
  model re-exports them); `crate::diagnostics::readback::doll`, the doll's readback check. The
  arsenal's preview in `apps/website/frontend/src/v2/apps/editor/arsenal/doll.rs` reaches it only
  through `crate::doll`.
- Rules: picking and rendering share one view-projection, so a pick lands on the pixel that was
  drawn: `view_proj_wgpu` is `view_proj_gl` with the depth remap in front and nothing else.
