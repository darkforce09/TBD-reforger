//! The even-odd scanline fill of a polygon over a regular sample grid.
//!
//! **Role:** [`SampleGrid`] places a grid of samples in world space and answers which rows a
//! vertical extent touches ([`SampleGrid::row_range`]) and which columns a horizontal span covers
//! ([`SampleGrid::span_columns`]); [`row_crossings`] gives where a ring's edges cross one row.
//! **Position:** the water export image lane fills lakes and ponds with it: for each row of
//! [`SampleGrid::row_range`] it takes the crossings at [`SampleGrid::row_z`], pairs them `(0, 1)`,
//! `(2, 3)`, … (an odd last crossing is ignored) and fills [`SampleGrid::span_columns`] of each
//! pair.
//! **Signals & state:** none; pure functions and a plain `Copy` value.
//! **Invariants:** sample `(column, row)` sits at `origin + index × spacing` on each axis; an edge
//! crosses a row when one end is at or below the row and the other strictly above (half-open), so
//! a vertex on the row counts once per side change and a horizontal edge never counts; a range
//! starts at the floor and ends at the ceiling of the grid coordinate, clamped to the grid, so a
//! span covers every sample it touches plus the one beyond each fractional end; a NaN bound gives
//! a range that holds no index.

use std::ops::RangeInclusive;

/// A regular grid of world-space samples: sample `(column, row)` sits at
/// `(origin_x + column × spacing_x, origin_z + row × spacing_z)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampleGrid {
    /// World x of column 0.
    pub origin_x: f64,
    /// World z of row 0.
    pub origin_z: f64,
    /// World distance between two columns.
    pub spacing_x: f64,
    /// World distance between two rows.
    pub spacing_z: f64,
    /// Number of columns.
    pub width: usize,
    /// Number of rows.
    pub height: usize,
}

impl SampleGrid {
    /// The rows from the floor of `min_z`'s row coordinate to the ceiling of `max_z`'s, clamped to
    /// the grid; `None` when the clamped start lies past the clamped end (the extent misses the
    /// grid). A NaN bound gives `Some` of a range that holds no row.
    #[must_use]
    pub fn row_range(&self, min_z: f64, max_z: f64) -> Option<RangeInclusive<usize>> {
        let (start, end) = clamped_bounds(
            (min_z - self.origin_z) / self.spacing_z,
            (max_z - self.origin_z) / self.spacing_z,
            self.height,
        );
        if start > end {
            return None;
        }
        Some(index_range(start, end))
    }

    /// The world z of `row`.
    #[must_use]
    pub fn row_z(&self, row: usize) -> f64 {
        self.origin_z + row as f64 * self.spacing_z
    }

    /// The world x of `column`.
    #[must_use]
    pub fn column_x(&self, column: usize) -> f64 {
        self.origin_x + column as f64 * self.spacing_x
    }

    /// The columns a span from `x_start` to `x_end` covers: from the floor of the start's column
    /// coordinate to the ceiling of the end's, clamped to the grid. `None` when the span ends left
    /// of column 0 or starts right of the last column; otherwise the range, which holds no column
    /// when the clamped start lies past the clamped end or a bound is NaN.
    #[must_use]
    pub fn span_columns(&self, x_start: f64, x_end: f64) -> Option<RangeInclusive<usize>> {
        let last_column_x = self.origin_x + last_index(self.width) * self.spacing_x;
        if x_end < self.origin_x || x_start > last_column_x {
            return None;
        }
        let (start, end) = clamped_bounds(
            (x_start - self.origin_x) / self.spacing_x,
            (x_end - self.origin_x) / self.spacing_x,
            self.width,
        );
        Some(index_range(start, end))
    }
}

/// The last index of `count` samples as an f64 (`-1.0` for an empty axis).
fn last_index(count: usize) -> f64 {
    count as f64 - 1.0
}

/// `(max(0, floor(start)), min(count − 1, ceil(end)))` of two grid coordinates, each NaN when its
/// coordinate is NaN. The NaN is kept explicitly because `f64::max` and `f64::min` drop a NaN
/// operand, and a NaN bound must select no index.
fn clamped_bounds(start_coordinate: f64, end_coordinate: f64, count: usize) -> (f64, f64) {
    let start = if start_coordinate.is_nan() {
        f64::NAN
    } else {
        start_coordinate.floor().max(0.0)
    };
    let end = if end_coordinate.is_nan() {
        f64::NAN
    } else {
        end_coordinate.ceil().min(last_index(count))
    };
    (start, end)
}

/// `start..=end` over clamped grid bounds, or a range holding no index when either is NaN, the
/// start lies past the end, or the end is negative.
fn index_range(start: f64, end: f64) -> RangeInclusive<usize> {
    if start.is_nan() || end.is_nan() || start > end || end < 0.0 {
        return empty_index_range();
    }
    // Both bounds are integral and non-negative here; the casts saturate at `usize::MAX`.
    (start as usize)..=(end as usize)
}

/// A range that holds no index.
#[allow(clippy::reversed_empty_ranges)]
fn empty_index_range() -> RangeInclusive<usize> {
    1..=0
}

/// The x of every crossing of the closed ring `ring` (points `[x, z]`) with the row at `z`, sorted
/// ascending. Edge `(ring[j], ring[j + 1])`, the last closing back to `ring[0]`, crosses when one
/// end's z is at or below the row and the other's strictly above; the crossing x is
/// `ax + t × (bx − ax)` with `t = (z − az) / (bz − az)`.
#[must_use]
pub fn row_crossings(ring: &[[f64; 2]], z: f64) -> Vec<f64> {
    let count = ring.len();
    let mut crossings = Vec::new();
    for index in 0..count {
        let [ax, az] = ring[index];
        let [bx, bz] = ring[(index + 1) % count];
        if (az <= z && bz > z) || (bz <= z && az > z) {
            let t = (z - az) / (bz - az);
            crossings.push(ax + t * (bx - ax));
        }
    }
    crossings.sort_by(f64::total_cmp);
    crossings
}

#[cfg(test)]
#[path = "tests/polygon_scanline_tests.rs"]
mod tests;
