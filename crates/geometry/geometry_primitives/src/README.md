# Geometry primitives source

The source of `geometry_primitives`: four pure geometry modules and the crate root that declares
them.

## Contents

```text
crates/geometry/geometry_primitives/src/
├── axis_aligned_box.rs  `Bounds3`: an axis-aligned 3D box, its union and its f32-rounded copy
├── lib.rs               the crate root: module header, `mod` lines and re-exports
├── prelude.rs           the names most callers import
├── rigid_transform.rs   `Rigid`: rotation, translation and uniform scale, and Enfusion angle conversion
├── segment_geometry.rs  the point along a 3D segment; 2D distances, segment intersections, segment-box entry
├── tests/               unit tests, one file per tested module
└── vector3.rs           `sub`, `cross` and `dot` of 3D vectors
```

## How it works

Every module is a set of pure functions or a plain `Copy` value over f64 arrays.
`segment_geometry::segment_intersects_aabb_2d` builds on `segment_aabb_entry_t_2d`, which tests
the segment against the box's four edges with `segment_intersection_t_2d`;
`rigid_transform::Rigid::from_enfusion` composes the three single-axis rotations yaw, pitch, roll.

## Boundaries

- Depends on: `serde`.
- Used by: the map engine (see the crate README).
- Rules: tests live in `tests/`, declared through `#[path]` by the module they test.
