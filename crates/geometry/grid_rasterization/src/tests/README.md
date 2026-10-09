# Grid rasterization tests

Unit tests of `grid_rasterization`, each file declared by the module it tests through `#[path]`.

## Contents

```text
crates/geometry/grid_rasterization/src/tests/
├── catmull_rom_tests.rs       segment ends, start tangent, straight lines, zero and NaN plan tangents
├── disc_stamping_tests.rs     disc coverage and feathering, visit order, clipping, stamp centre counts
├── half_up_rounding_tests.rs  ties, negative zero, the value below a half, large and non-finite values
└── polygon_scanline_tests.rs  crossings of square, concave and diamond rings; row ranges and span clipping
```

## Boundaries

- Depends on: the module each file tests (`crate::<module>`).
- Used by: `cargo test -p grid_rasterization`.
- Rules: every expected value is written out by hand.
