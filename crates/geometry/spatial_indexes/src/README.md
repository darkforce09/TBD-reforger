# Spatial indexes source

The source of `spatial_indexes`: the bounding volume hierarchy, the point indexes, the crate's
error and prelude, and the test fixtures other crates' tests share.

## Contents

```text
crates/geometry/spatial_indexes/src/
├── bounding_volume_hierarchy/  the flat-tree build core, the triangle tree, its sidecar format and the segment tests
├── error.rs                    `Error` and `Result`: the sidecar parse refusals, wrapped
├── lib.rs                      the crate root: module header, `mod` lines and the re-exports
├── point_indexes/              the point grid, the selection picks and the zoom-level clusters
├── prelude.rs                  the common names for glob import
└── test_fixtures.rs            cuboid scenes for BVH tests, behind `cfg(test)` or the `test_fixtures` feature
```

## How it works

`lib.rs` declares the two module folders, the error and the prelude, and compiles `test_fixtures`
only for this crate's tests and the `test_fixtures` feature.

## Boundaries

- Depends on: `geometry_primitives`, `thiserror`.
- Used by: the map engine, the single-page app and the developer tools, through the crate root.
- Rules: `lib.rs` holds only the module header, `mod` lines and `pub use` lines
  (`cargo xtask verify crate-anatomy`).
