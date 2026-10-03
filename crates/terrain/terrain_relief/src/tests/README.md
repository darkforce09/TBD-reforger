# Terrain relief tests

Unit tests of `terrain_relief`, one file per module, each declared by its module through
`#[path]`.

## Contents

```text
crates/terrain/terrain_relief/src/tests/
├── contours_tests.rs   reductions and levels, marching, ring closure and the summit pick
├── hillshade_tests.rs  decimation, flat ground and the sun constants
└── sea_band_tests.rs   the opacity ladder, dry land, whole-run rectangles and ring closure
```

## Boundaries

- Depends on: the module each file tests (`crate::<module>`) and `terrain_elevation`'s vector
  grid.
- Used by: `cargo test -p terrain_relief`.
- Rules: the grids are built by hand in each file; no file is read.
