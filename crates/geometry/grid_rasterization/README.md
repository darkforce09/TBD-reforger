# Grid rasterization

The `grid_rasterization` crate: the arithmetic that turns vector shapes into the samples of a
regular grid, with no map, world, file or GPU concept. It holds rounding with ties toward +∞, the
uniform Catmull-Rom spline with its plan-view tangent and normal, the even-odd scanline fill of a
polygon over a sample grid, and the anti-aliased disc stamps that stroke a segment on a pixel
canvas.

## Contents

```text
crates/geometry/grid_rasterization/
├── Cargo.toml  the package: no dependency, layout tier 0
└── src/        the rounding, the spline, the polygon scanline, the disc stamps and the prelude
```

## How it works

`round_half_up` rounds to the nearest integer with a tie going toward +∞ (`-2.5` gives `-2`,
where `f64::round` gives `-3`), and is exact for every f64, `0.49999999999999994` included.

`evaluate_uniform_catmull_rom` takes four `[x, y, z]` control points and a parameter `t` and
returns a `SplineSample`: the point between the middle two, the curve's unit direction in the XZ
plane and that direction turned a quarter turn. A zero or NaN plan direction is divided by one
instead of its length.

`SampleGrid` places samples at `origin + index × spacing`. A polygon fill takes the rows of
`row_range` over its extent, the sorted `row_crossings` of its ring at each row's `row_z` (an edge
counts when one end is at or below the row and the other strictly above), pairs them `(0, 1)`,
`(2, 3)`, … and fills the `span_columns` of each pair. A range runs from the floor to the ceiling
of the grid coordinate, clamped to the grid; a span that misses the grid is `None`.

`disc_coverage` visits every pixel a disc covers, row by row and left to right, with the full
alpha inside and a 0.75 px feathered rim, skipping zero coverage. `segment_stamp_centres` spaces
centres at most half a pixel apart along a segment (`max(2, ceil(length × 2))` steps), or returns
the start alone for a segment under half a pixel; stamping a disc at each strokes the segment.

Every expression keeps one evaluation order with no fused multiply-add, so the samples are the
same to the bit on every target.

## Getting started

Run from the repository root:

```bash
cargo test -p grid_rasterization   # rounding, spline, scanline and disc stamp unit tests
```

## Public surface

- `half_up_rounding::round_half_up`.
- `catmull_rom::{SplineSample, evaluate_uniform_catmull_rom}`.
- `polygon_scanline::{SampleGrid, row_crossings}`, with `SampleGrid::{row_range, row_z,
  column_x, span_columns}`.
- `disc_stamping::{disc_coverage, segment_stamp_centres}`.
- `prelude`, which re-exports every item above.

## Boundaries

- Depends on: nothing.
- Used by: the map raster pipeline's water and road export image lanes
  (`tools/map_assets/map_raster_pipeline`).
- Rules: no map, world, file or GPU concept enters this crate; a tie rounds toward +∞
  (`a_negative_tie_rounds_toward_positive_infinity` in `src/tests/half_up_rounding_tests.rs`); a
  vertex on a row counts once per side change (`a_vertex_on_the_row_counts_once_per_side`); disc
  visits run row by row (`a_radius_two_disc_on_a_pixel_feathers_its_diagonals_and_drops_its_rim`);
  geometry tier 0, no workspace dependency (`cargo xtask verify crate-tiers`).
