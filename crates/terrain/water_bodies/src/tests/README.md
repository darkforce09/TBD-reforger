# Water bodies tests

Unit tests of `water_bodies`, declared by `vectors.rs` and `bathymetry_palette.rs` through
`#[path]`.

## Contents

```text
crates/terrain/water_bodies/src/tests/
├── bathymetry_palette_tests.rs  ramp stops, interpolation, clamping, contour lines and mask byte classes
└── vectors_tests.rs             bathymetry levels, the mask, the suffix plan and the inland water archive
```

## Boundaries

- Depends on: `crate::vectors`, `crate::bathymetry_palette`, and `world_file_formats` to frame the containers and write the
  archives.
- Used by: `cargo test -p water_bodies`.
- Rules: the containers and archives are built in memory; no file is read.
