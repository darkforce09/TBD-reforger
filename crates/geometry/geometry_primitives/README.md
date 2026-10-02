# Geometry primitives

The `geometry_primitives` crate: plain f64 geometry with no map, world or GPU concept. It holds the
3D vector products of the ray–triangle test, the 2D segment and box tests of building
line-of-sight attribution, the rigid transform of every placed building part, and the
axis-aligned 3D box of an occluder descriptor.

## Contents

```text
crates/geometry/geometry_primitives/
├── Cargo.toml  the package: `serde`, layout tier 0
└── src/        the vector products, segment tests, rigid transform, 3D box and prelude
```

## How it works

Points, vectors and boxes are plain `[f64; 2]` and `[f64; 3]` arrays, so a caller passes its own
coordinates without conversion. `vector3` holds `sub`, `cross` and `dot`. `segment_geometry`
holds `point_at` (the point at a parameter along a 3D segment) and the plan-view tests: distances
from a point to a segment, the intersection of two segments with its parameter, and where a
segment enters a box. `rigid_transform::Rigid` is a rotation matrix, a translation and a uniform
scale: built from Enfusion `[pitch, yaw, roll]` angles or a quaternion, composed, inverted and
applied to points, directions and boxes, and read back as a quaternion or a plan heading.
`axis_aligned_box::Bounds3` is a box by its corners, with a union and an f32-rounded copy; it
derives `serde` because the occluder manifest stores it as JSON.

## Getting started

Run from the repository root:

```bash
cargo test -p geometry_primitives   # the rigid transform and box unit tests
```

## Public surface

- `vector3::{sub, cross, dot}`.
- `segment_geometry::{point_at, dist_2d, point_segment_dist_2d, aabb_contains_2d,
  segment_intersection_t_2d, line_segment_intersection_2d, segment_aabb_entry_t_2d,
  segment_intersects_aabb_2d}`.
- `rigid_transform::Rigid`.
- `axis_aligned_box::Bounds3`.
- `prelude`, which re-exports every item above.

## Boundaries

- Depends on: `serde` (the box's JSON form).
- Used by: the map engine (`legacy/map_engine`), from its `bvh` tier: the BVH node test and
  traversal and the section cutter (`vector3`), the building blueprints and the interior and
  world line-of-sight walkers (`segment_geometry`), the compound buildings (`rigid_transform`)
  and the occluder descriptors (`axis_aligned_box`); the developer tools (`tools/developer_tools`):
  the blueprint compiler and the world line-of-sight checks; and the single-page app's building
  interior bench (`rigid_transform`).
- Rules: no map, world, terrain or GPU concept enters this crate; a rigid transform keeps an
  orthonormal rotation, so its inverse is exact to rounding
  (`rot_y_turns_x_toward_z_and_inverse_undoes`); geometry tier 0, no workspace dependency
  (`cargo xtask verify crate-tiers`).

## Related documentation

- [Map engine overview](/documentation/legacy/map_engine/map_engine_overview.md) — the map
  engine's tiers and the modules that call these primitives.
