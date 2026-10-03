# Terrain elevation tests

Unit tests of `terrain_elevation`, one file per module, each declared by its module through
`#[path]`.

## Contents

```text
crates/terrain/terrain_elevation/src/tests/
├── full_resolution_tests.rs  the native raster's frame, encoding, refusals and agreement with the vector grid
├── grid_tests.rs             vector grid dimensions, sampling, the box average and the 2× reduction
├── png_tests.rs              16-bit PNG samples and metres, and the refusal of bytes that are not a PNG
├── raw_tests.rs              raw grid decode whole and streamed, refusals, and agreement with the PNG
└── sampling_tests.rs         sample to metres, world to pixel, bilinear sampling, the off-raster point
```

## Boundaries

- Depends on: the module each file tests (`crate::<module>`), and the `png` crate to encode
  inputs.
- Used by: `cargo test -p terrain_elevation`.
- Rules: the cases keep their assertions and fixtures; inputs are built in memory, no file is read.
