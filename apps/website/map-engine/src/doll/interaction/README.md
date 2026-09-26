# Doll picking and callout anchors

The pointer math of the doll preview in the [arsenal](/documentation_v2/glossary/a_to_f.md#arsenal): which
clickable part of the mannequin lies under a pixel, and where each part's callout anchors on
screen. Pure functions over the scene model and the orbit camera, with no GPU.

## Contents

```text
apps/website/map-engine/src/doll/interaction/
├── mod.rs      the module tree
└── picking.rs  `pick`, `anchor_world` and `anchor_px`: ray picking and callout anchors
```

## How it works

`pick(yaw, w_px, h_px, x_px, y_px)` inverts `view_proj_gl` for the viewport, turns the pixel into
a ray between the near and far planes, and moves that ray into each part's model space, where
every part is a unit box of extent ±0.5 (the slab test in `ray_unit_box`). The nearest hit among
the parts with a region index of 0 or more wins, and the result is that index into `REGION_KEYS`,
or -1 for a miss, a zero-sized viewport or a matrix that does not invert. Body decor (`DECOR`)
never takes a pick. The launcher's cylinder is tested as its bounding box.

`anchor_world(region)` is the translation of the region's first instance, the centre of that part.
`anchor_px` projects it with the same `view_proj_gl` into CSS pixels, top-left origin, and returns
`None` for an unknown region, a zero-sized viewport or a point behind the camera (clip w ≤ 0), so
the caller hides the callout.

## Boundaries

- Depends on: `crate::camera::math::glmat4` (`invert`, `transform_vector`),
  `crate::camera::orbit::projection::view_proj_gl` and `crate::doll::scene::instances`.
- Used by: `crate::doll::renderer`, whose `DollEngine::pick_region` and `DollEngine::anchor_px`
  call these with the engine's yaw and CSS size; `crate::doll::scene::model`, which re-exports all
  three functions for the model tests.
- Rules: picking inverts the same view-projection the renderer draws with, so a pick lands on the
  drawn pixel (`pick_goldens_center_regions` and `pick_yaw_symmetry_hits_backpack_from_behind` in
  `apps/website/map-engine/src/doll/scene/model/tests/cases_1.rs`); every region's anchor projects
  inside an 800 × 600 viewport at yaw 0 and matches a direct projection
  (`anchors_every_region_projects_inside_the_viewport`,
  `anchor_matches_transform_vector_projection`, same file).

## Related documentation

- [Orbit camera](/apps/website/map-engine/src/camera/orbit/README.md) — the fixed-orbit camera
  whose `view_proj_gl` both functions invert or apply.
