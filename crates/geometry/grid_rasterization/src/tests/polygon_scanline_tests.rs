//! **Role:** unit tests of [`crate::polygon_scanline`]: the crossing rule over square, concave and
//! diamond rings, row ranges and span columns.
//! **Position:** `src/tests` of `grid_rasterization`, declared by `polygon_scanline.rs`.
//! **Signals & state:** none; pure functions over literal rings and grids.
//! **Invariants:** every expected value is written out by hand, never derived from the code under
//! test.

use super::{SampleGrid, row_crossings};

/// Ten by ten samples one metre apart from the world origin.
const UNIT_GRID: SampleGrid = SampleGrid {
    origin_x: 0.0,
    origin_z: 0.0,
    spacing_x: 1.0,
    spacing_z: 1.0,
    width: 10,
    height: 10,
};

#[test]
fn a_square_crosses_each_row_at_its_two_sides() {
    let square = [[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
    assert_eq!(row_crossings(&square, 2.0), vec![0.0, 4.0]);
    // The bottom edge is horizontal and never counts; both sides start on the row.
    assert_eq!(row_crossings(&square, 0.0), vec![0.0, 4.0]);
    // The top row is excluded: neither side lies strictly above it.
    assert!(row_crossings(&square, 4.0).is_empty());
}

#[test]
fn a_concave_ring_gives_sorted_crossing_pairs() {
    let u_shape = [
        [0.0, 0.0],
        [6.0, 0.0],
        [6.0, 6.0],
        [4.0, 6.0],
        [4.0, 2.0],
        [2.0, 2.0],
        [2.0, 6.0],
        [0.0, 6.0],
    ];
    assert_eq!(row_crossings(&u_shape, 4.0), vec![0.0, 2.0, 4.0, 6.0]);
    assert_eq!(row_crossings(&u_shape, 1.0), vec![0.0, 6.0]);
}

#[test]
fn a_vertex_on_the_row_counts_once_per_side() {
    let diamond = [[2.0, 0.0], [4.0, 2.0], [2.0, 4.0], [0.0, 2.0]];
    assert_eq!(row_crossings(&diamond, 2.0), vec![0.0, 4.0]);
    // The bottom vertex sits on the row: both edges leaving it upward count, at its x.
    assert_eq!(row_crossings(&diamond, 0.0), vec![2.0, 2.0]);
    // The top vertex has no edge strictly above it.
    assert!(row_crossings(&diamond, 4.0).is_empty());
}

#[test]
fn a_crossing_interpolates_along_its_edge() {
    let triangle = [[0.0, 0.0], [8.0, 0.0], [0.0, 4.0]];
    assert_eq!(row_crossings(&triangle, 1.0), vec![0.0, 6.0]);
}

#[test]
fn row_range_spans_floor_to_ceiling_clamped_to_the_grid() {
    assert_eq!(UNIT_GRID.row_range(2.5, 4.2), Some(2..=5));
    assert_eq!(UNIT_GRID.row_range(-3.0, 20.0), Some(0..=9));
    assert_eq!(UNIT_GRID.row_range(3.0, 3.0), Some(3..=3));
}

#[test]
fn row_range_misses_an_extent_off_the_grid() {
    assert_eq!(UNIT_GRID.row_range(12.0, 15.0), None);
    assert_eq!(UNIT_GRID.row_range(-5.0, -2.0), None);
    let empty_grid = SampleGrid {
        height: 0,
        ..UNIT_GRID
    };
    assert_eq!(empty_grid.row_range(0.0, 1.0), None);
}

#[test]
fn row_range_with_a_nan_bound_holds_no_row() {
    let range = UNIT_GRID.row_range(f64::NAN, 3.0);
    assert!(range.is_some_and(|rows| rows.is_empty()));
    let range = UNIT_GRID.row_range(1.0, f64::NAN);
    assert!(range.is_some_and(|rows| rows.is_empty()));
}

#[test]
fn row_and_column_positions_follow_origin_and_spacing() {
    let grid = SampleGrid {
        origin_x: 100.0,
        origin_z: -50.0,
        spacing_x: 2.0,
        spacing_z: 0.5,
        width: 4,
        height: 4,
    };
    assert_eq!(grid.column_x(3), 106.0);
    assert_eq!(grid.row_z(3), -48.5);
    assert_eq!(grid.row_range(-49.2, -48.9), Some(1..=3));
}

#[test]
fn span_columns_inside_the_grid_take_floor_and_ceiling() {
    assert_eq!(UNIT_GRID.span_columns(2.5, 4.2), Some(2..=5));
    assert_eq!(UNIT_GRID.span_columns(3.0, 3.0), Some(3..=3));
}

#[test]
fn span_columns_clip_at_both_edges() {
    assert_eq!(UNIT_GRID.span_columns(-5.0, 3.5), Some(0..=4));
    assert_eq!(UNIT_GRID.span_columns(7.5, 20.0), Some(7..=9));
    assert_eq!(UNIT_GRID.span_columns(9.0, 12.0), Some(9..=9));
}

#[test]
fn span_columns_off_the_grid_are_skipped() {
    assert_eq!(UNIT_GRID.span_columns(-5.0, -1.0), None);
    assert_eq!(UNIT_GRID.span_columns(9.5, 12.0), None);
}

#[test]
fn a_reversed_or_nan_span_holds_no_column() {
    let reversed = UNIT_GRID.span_columns(5.5, 3.2);
    assert!(reversed.is_some_and(|columns| columns.is_empty()));
    let nan = UNIT_GRID.span_columns(f64::NAN, 3.0);
    assert!(nan.is_some_and(|columns| columns.is_empty()));
}
