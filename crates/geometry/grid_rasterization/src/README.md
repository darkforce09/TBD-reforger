# Grid rasterization source

The source of `grid_rasterization`: four pure rasterization modules, the prelude and the crate root
that declares them.

## Contents

```text
crates/geometry/grid_rasterization/src/
├── catmull_rom.rs       `evaluate_uniform_catmull_rom`: the point, plan tangent and normal of a uniform spline
├── disc_stamping.rs     `disc_coverage` and `segment_stamp_centres`: anti-aliased discs that stroke a segment
├── half_up_rounding.rs  `round_half_up`: the nearest integer, ties toward +∞
├── lib.rs               the crate root: module header and `mod` lines
├── polygon_scanline.rs  `SampleGrid` and `row_crossings`: the even-odd scanline fill over a sample grid
├── prelude.rs           the names most callers import
└── tests/               unit tests, one file per tested module
```

## How it works

Every module is a set of pure functions over f64 values or a plain `Copy` value. The scanline and
the disc stamps hand out indices and coverage; the caller owns the grid or canvas and writes it.

## Boundaries

- Depends on: nothing.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: tests live in `tests/`, declared through `#[path]` by the module they test.
