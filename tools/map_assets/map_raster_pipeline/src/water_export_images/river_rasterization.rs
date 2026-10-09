//! The ribbon stamp of the export's river splines.
//!
//! **Role:** [`ribbon_rivers`] samples every river's centre line as a uniform Catmull-Rom spline
//! and stamps a cross-section along the plan normal at each sample: every stamped sample becomes
//! river and is deepened to a parabolic channel depth.
//! **Position:** called by the vector rasterizer after the lakes and ponds, so a river crossing a
//! lake marks its samples as river.
//! **Signals & state:** none held; writes the caller's [`WaterRaster`].
//! **Invariants:** the centre line is `splinePoints` when given, otherwise each of `nodes` (a point
//! array, or a node's `pos`), and a river of fewer than two points draws nothing; a segment's
//! neighbours clamp to its own ends at the ends of the line; the river width is `widthM`, else
//! `averageWidthM`, else 6 m, and node `i`'s `widthM` and `depthM` (else 1.4 m) override it at
//! that node — node fields count when given, as non-zero numbers; the sampling step is the smallest
//! of the two sample spacings and 0.5 m, a segment takes `max(4, ceil(length / step))` steps and
//! a half width `max(2, ceil(half width / step))` cross steps each side; a stamped point rounds to
//! the nearest sample, ties toward +∞, and is dropped off the grid; a cross-section point at
//! `ratio` of the half width from the centre is `max(0.25, 1 − ratio²)` of the centre depth;
//! every segment counts as drawn, even one whose length is NaN and takes no step; a step count
//! that is infinite (a zero sample spacing) is refused rather than looped forever.

use grid_rasterization::prelude::{evaluate_uniform_catmull_rom, round_half_up};
use serde_json::Value;

use super::json_field_reading::{coordinate, given_number, is_given};
use super::water_raster::{WaterRaster, nan_propagating_max, nan_propagating_min};
use crate::error::{Result, bail};

/// The water class byte of a river sample.
const RIVER_CLASS_CODE: u8 = 3;

/// The width of a river that gives none, in metres.
const FALLBACK_RIVER_WIDTH_M: f64 = 6.0;

/// The centre depth at a node that gives none, in metres.
const FALLBACK_RIVER_DEPTH_M: f64 = 1.4;

/// The longest sampling step along and across a river, in metres.
const MAXIMUM_RIVER_STEP_M: f64 = 0.5;

/// The fewest steps a segment takes.
const MINIMUM_SEGMENT_STEPS: f64 = 4.0;

/// The fewest cross steps on each side of the centre line.
const MINIMUM_CROSS_STEPS: f64 = 2.0;

/// The shallowest share of the centre depth at a channel edge.
const MINIMUM_CROSS_SECTION_FACTOR: f64 = 0.25;

/// Stamps every river of `rivers` (a JSON array; anything else draws nothing) into `raster`;
/// returns how many segments were drawn. Refuses a sampling step of zero, which would take
/// infinitely many steps.
pub(super) fn ribbon_rivers(rivers: &Value, raster: &mut WaterRaster) -> Result<usize> {
    let Some(rivers) = rivers.as_array() else {
        return Ok(0);
    };
    let grid = raster.grid;
    let step = nan_propagating_min(
        nan_propagating_min(grid.spacing_x, grid.spacing_z),
        MAXIMUM_RIVER_STEP_M,
    );
    let mut drawn_segments = 0;
    for river in rivers {
        let points = centre_line(river);
        if points.len() < 2 {
            continue;
        }
        let nodes = river.get("nodes").and_then(Value::as_array);
        let node_field = |index: usize, key: &str| {
            given_number(
                nodes
                    .and_then(|nodes| nodes.get(index))
                    .and_then(|node| node.get(key)),
            )
        };
        let default_width = given_number(river.get("widthM"))
            .or_else(|| given_number(river.get("averageWidthM")))
            .unwrap_or(FALLBACK_RIVER_WIDTH_M);
        for index in 0..points.len() - 1 {
            let segment = RiverSegment {
                controls: [
                    points[index.saturating_sub(1)],
                    points[index],
                    points[index + 1],
                    points[(index + 2).min(points.len() - 1)],
                ],
                widths_m: [
                    node_field(index, "widthM").unwrap_or(default_width),
                    node_field(index + 1, "widthM").unwrap_or(default_width),
                ],
                depths_m: [
                    node_field(index, "depthM").unwrap_or(FALLBACK_RIVER_DEPTH_M),
                    node_field(index + 1, "depthM").unwrap_or(FALLBACK_RIVER_DEPTH_M),
                ],
            };
            segment.stamp(step, raster)?;
            drawn_segments += 1;
        }
    }
    Ok(drawn_segments)
}

/// The centre line of `river`: `splinePoints` when given, otherwise the points of `nodes`; a
/// given value that is not an array gives no point.
fn centre_line(river: &Value) -> Vec<[f64; 3]> {
    if let Some(spline) = river.get("splinePoints").filter(|value| is_given(value)) {
        return spline
            .as_array()
            .map(|points| points.iter().map(|point| point_of(Some(point))).collect())
            .unwrap_or_default();
    }
    river
        .get("nodes")
        .and_then(Value::as_array)
        .map(|nodes| {
            nodes
                .iter()
                .map(|node| {
                    if node.is_array() {
                        point_of(Some(node))
                    } else {
                        point_of(node.get("pos"))
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The `[x, y, z]` of a point array, NaN where a coordinate is missing.
fn point_of(point: Option<&Value>) -> [f64; 3] {
    [
        coordinate(point, 0),
        coordinate(point, 1),
        coordinate(point, 2),
    ]
}

/// One segment of a river centre line with its end widths and depths.
struct RiverSegment {
    /// The four spline control points; the segment runs from the second to the third.
    controls: [[f64; 3]; 4],
    /// The river width at the segment's start and end, in metres.
    widths_m: [f64; 2],
    /// The centre depth at the segment's start and end, in metres.
    depths_m: [f64; 2],
}

impl RiverSegment {
    /// Stamps the segment's cross-sections into `raster` every `step` metres.
    fn stamp(&self, step: f64, raster: &mut WaterRaster) -> Result<()> {
        let [p0, p1, p2, p3] = self.controls;
        let length = (p2[0] - p1[0]).hypot(p2[2] - p1[2]);
        let steps = nan_propagating_max(MINIMUM_SEGMENT_STEPS, (length / step).ceil());
        refuse_infinite(steps)?;
        let [start_width, end_width] = self.widths_m;
        let [start_depth, end_depth] = self.depths_m;
        let mut step_index = 0.0;
        while step_index <= steps {
            let t = step_index / steps;
            let sample = evaluate_uniform_catmull_rom(p0, p1, p2, p3, t);
            let width = start_width + (end_width - start_width) * t;
            let centre_depth = start_depth + (end_depth - start_depth) * t;
            let half_width = width * 0.5;
            let cross_steps = nan_propagating_max(MINIMUM_CROSS_STEPS, (half_width / step).ceil());
            refuse_infinite(cross_steps)?;
            let mut cross_index = -cross_steps;
            while cross_index <= cross_steps {
                let ratio = cross_index / cross_steps;
                let offset = ratio * half_width;
                let x = sample.position[0] + sample.normal[0] * offset;
                let z = sample.position[2] + sample.normal[2] * offset;
                let factor = nan_propagating_max(MINIMUM_CROSS_SECTION_FACTOR, 1.0 - ratio * ratio);
                stamp_point(x, z, centre_depth * factor, raster);
                cross_index += 1.0;
            }
            step_index += 1.0;
        }
        Ok(())
    }
}

/// Marks the sample nearest world `(x, z)` as river and deepens it to `depth_m`; a point off the
/// grid stamps nothing.
fn stamp_point(x: f64, z: f64, depth_m: f64, raster: &mut WaterRaster) {
    let grid = raster.grid;
    let column = round_half_up((x - grid.origin_x) / grid.spacing_x);
    let row = round_half_up((z - grid.origin_z) / grid.spacing_z);
    let on_grid =
        column >= 0.0 && column < grid.width as f64 && row >= 0.0 && row < grid.height as f64;
    if !on_grid {
        return;
    }
    // Whole numbers inside the grid, so the casts are exact.
    let index = raster.index(column as usize, row as usize);
    raster.mask[index] = RIVER_CLASS_CODE;
    raster.deepen(index, depth_m);
}

/// Refuses an infinite step count.
fn refuse_infinite(steps: f64) -> Result<()> {
    if steps.is_infinite() {
        bail!("a river sampling step of zero metres would take infinitely many steps");
    }
    Ok(())
}
