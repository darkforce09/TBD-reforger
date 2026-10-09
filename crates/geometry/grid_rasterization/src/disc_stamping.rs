//! Anti-aliased disc stamps on a pixel canvas, and the stamp centres that stroke a segment.
//!
//! **Role:** [`disc_coverage`] visits every pixel a disc covers with the alpha the disc lends it,
//! feathered over the outer 0.75 px; [`segment_stamp_centres`] spaces disc centres along a segment
//! at most half a pixel apart, so a run of discs strokes it.
//! **Position:** the road export image lane strokes each road polyline with these stamps and blends
//! every visit into its canvas.
//! **Signals & state:** none; pure functions, the canvas stays with the caller's visitor.
//! **Invariants:** pixel `(x, y)` is the sample at integer `(x, y)`; a pixel is covered when its
//! squared distance to the centre is at most the squared radius; visits run row by row, top row
//! first and left to right within a row, the order a blending caller depends on; a coverage alpha
//! of zero is never visited; NaN or out-of-canvas geometry visits nothing.

/// The width, in pixels, of the feathered rim inside a disc's edge.
const FEATHER_WIDTH_PX: f64 = 0.75;

/// Visits every pixel of a `width` × `height` canvas that the disc at (`centre_x`, `centre_y`) of
/// `radius` pixels covers, as `visit(x, y, coverage_alpha)`, rows top to bottom and columns left to
/// right.
///
/// The scanned box runs from `floor(centre − ceil(radius))` to `ceil(centre + ceil(radius))` on
/// each axis, clamped to the canvas. A pixel at distance `d` with `d² ≤ radius²` gets `alpha`, or
/// `round(alpha × (radius − d) / 0.75)` within 0.75 px of the edge; a pixel whose coverage rounds
/// to 0 is skipped. `radius − d` is never negative for a covered pixel (the square root of a
/// correctly rounded square is the value itself, and the root is monotonic), so the product is
/// non-negative and [`f64::round`] rounds it exactly as ties toward +∞ would.
pub fn disc_coverage(
    centre_x: f64,
    centre_y: f64,
    radius: f64,
    alpha: u8,
    width: usize,
    height: usize,
    mut visit: impl FnMut(usize, usize, u8),
) {
    let reach = radius.ceil();
    let squared_radius = radius * radius;
    let Some((first_column, last_column)) =
        clamped_pixel_span(centre_x - reach, centre_x + reach, width)
    else {
        return;
    };
    let Some((first_row, last_row)) =
        clamped_pixel_span(centre_y - reach, centre_y + reach, height)
    else {
        return;
    };
    for y in first_row..=last_row {
        let offset_y = y as f64 - centre_y;
        let squared_offset_y = offset_y * offset_y;
        for x in first_column..=last_column {
            let offset_x = x as f64 - centre_x;
            let squared_distance = offset_x * offset_x + squared_offset_y;
            if squared_distance <= squared_radius {
                let edge_distance = radius - squared_distance.sqrt();
                let coverage = if edge_distance < FEATHER_WIDTH_PX {
                    (f64::from(alpha) * (edge_distance / FEATHER_WIDTH_PX)).round()
                } else {
                    f64::from(alpha)
                };
                if coverage > 0.0 {
                    // `coverage` is an integer in `1..=alpha`, so the cast is exact.
                    visit(x, y, coverage as u8);
                }
            }
        }
    }
}

/// The pixel indices `max(0, floor(low))..=min(count − 1, ceil(high))`, or `None` when the span
/// misses the canvas or either bound is NaN.
fn clamped_pixel_span(low: f64, high: f64, count: usize) -> Option<(usize, usize)> {
    if low.is_nan() || high.is_nan() {
        return None;
    }
    let first = low.floor().max(0.0);
    let last = high.ceil().min(count as f64 - 1.0);
    if first > last || last < 0.0 {
        return None;
    }
    // Both bounds are integral and non-negative here; the casts saturate at `usize::MAX`.
    Some((first as usize, last as usize))
}

/// The disc centres that stroke the segment from (`x0`, `y0`) to (`x1`, `y1`): the start alone when
/// the segment is shorter than half a pixel, none when its length is NaN or infinite, otherwise
/// `steps + 1` evenly spaced points from start to end with `steps = max(2, ceil(length × 2))`,
/// point `s` at `start + delta × (s × (1 / steps))`.
#[must_use]
pub fn segment_stamp_centres(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    let delta_x = x1 - x0;
    let delta_y = y1 - y0;
    let length = delta_x.hypot(delta_y);
    if length < 0.5 {
        return vec![[x0, y0]];
    }
    if !length.is_finite() {
        return Vec::new();
    }
    let steps = (length * 2.0).ceil().max(2.0);
    let inverse_steps = 1.0 / steps;
    // `steps` is a finite integer of at least 2; the cast saturates past `usize::MAX`.
    let step_count = steps as usize;
    (0..=step_count)
        .map(|step| {
            let t = step as f64 * inverse_steps;
            [x0 + delta_x * t, y0 + delta_y * t]
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/disc_stamping_tests.rs"]
mod tests;
