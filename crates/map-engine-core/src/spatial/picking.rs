//! Role: spatial hit queries over row-aligned coordinate columns.
//! Position: graphics spatial indexing; independent of document identifiers.
//! Signals & state: explicit coordinates, radius, and grid resolution.
//! Invariants: square slot hits, circular point hits, stable ties and traversal order.

use super::point_index::PointIndex;

/// Find the closest row in a square query, retaining the first row on equal distances.
pub fn pick_slot_row(xs: &[f32], ys: &[f32], qx: f64, qy: f64, r: f64, cell_m: f64) -> Option<u32> {
    if xs.is_empty() || !qx.is_finite() || !qy.is_finite() {
        return None;
    }
    let idx = PointIndex::build(xs.to_vec(), ys.to_vec(), cell_m);
    let mut best: Option<(f64, u32)> = None;
    for h in idx.pick_rect(qx - r, qy - r, qx + r, qy + r) {
        let dx = f64::from(xs[h as usize]) - qx;
        let dy = f64::from(ys[h as usize]) - qy;
        let d2 = dx * dx + dy * dy;
        if best.is_none_or(|(bd, _)| d2 < bd) {
            best = Some((d2, h));
        }
    }
    best.map(|(_, h)| h)
}

/// Find the closest point in a circular query, retaining input order on ties.
pub fn pick_point_row(
    points: impl IntoIterator<Item = (f64, f64)>,
    qx: f64,
    qy: f64,
    r: f64,
) -> Option<usize> {
    if !qx.is_finite() || !qy.is_finite() {
        return None;
    }
    let r2 = r * r;
    let mut best: Option<(f64, usize)> = None;
    for (h, (x, y)) in points.into_iter().enumerate() {
        let dx = x - qx;
        let dy = y - qy;
        let d2 = dx * dx + dy * dy;
        if d2 > r2 {
            continue;
        }
        if best.is_none_or(|(bd, _)| d2 < bd) {
            best = Some((d2, h));
        }
    }
    best.map(|(_, h)| h)
}

/// Return rows in the point index's traversal order for a world-space rectangle.
pub fn marquee_slot_rows(
    xs: &[f32],
    ys: &[f32],
    start: [f64; 2],
    end: [f64; 2],
    cell_m: f64,
) -> Vec<u32> {
    if xs.is_empty() || !start.into_iter().chain(end).all(f64::is_finite) {
        return Vec::new();
    }
    let idx = PointIndex::build(xs.to_vec(), ys.to_vec(), cell_m);
    idx.pick_rect(
        start[0].min(end[0]),
        start[1].min(end[1]),
        start[0].max(end[0]),
        start[1].max(end[1]),
    )
}

/// Return point rows inside a world-space rectangle in input order.
pub fn marquee_point_rows(
    points: impl IntoIterator<Item = (f64, f64)>,
    start: [f64; 2],
    end: [f64; 2],
) -> Vec<usize> {
    if !start.into_iter().chain(end).all(f64::is_finite) {
        return Vec::new();
    }
    let (min_x, max_x) = (start[0].min(end[0]), start[0].max(end[0]));
    let (min_y, max_y) = (start[1].min(end[1]), start[1].max(end[1]));
    points
        .into_iter()
        .enumerate()
        .filter(|(_, (x, y))| *x >= min_x && *x <= max_x && *y >= min_y && *y <= max_y)
        .map(|(h, _)| h)
        .collect()
}
