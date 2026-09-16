//! Role: route geometry.
//! Position: `world/environment/locations` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::environment::locations::route_placement::ROAD_NAME_DECLUTTER_BASE_M;
use crate::world::environment::locations::route_placement::ROAD_NAME_LONG_SEGMENT_M;
use crate::world::environment::locations::route_placement::RoadLabelPlacement;

/// Declutter threshold in world meters at `deck_zoom`.
#[must_use]
pub fn road_declutter_min_dist_m(deck_zoom: f64) -> f64 {
    ROAD_NAME_DECLUTTER_BASE_M * 2f64.powf(-deck_zoom)
}

/// Total polyline arc length (m).
#[must_use]
pub fn polyline_length(points: &[[f64; 2]]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }
    let mut len = 0.0;
    for w in points.windows(2) {
        let dx = w[1][0] - w[0][0];
        let dy = w[1][1] - w[0][1];
        len += dx.hypot(dy);
    }
    len
}

/// Point + unit tangent at normalized arc fraction `frac` ∈ [0,1].
#[must_use]
pub fn point_tangent_at_frac(points: &[[f64; 2]], frac: f64) -> Option<([f64; 2], [f64; 2])> {
    if points.len() < 2 {
        return None;
    }
    let total = polyline_length(points);
    if total < 1e-6 {
        return None;
    }
    let target = frac.clamp(0.0, 1.0) * total;
    let mut acc = 0.0;
    for w in points.windows(2) {
        let a = w[0];
        let b = w[1];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let seg_len = dx.hypot(dy);
        if seg_len < 1e-12 {
            continue;
        }
        if acc + seg_len >= target {
            let t = ((target - acc) / seg_len).clamp(0.0, 1.0);
            let px = a[0] + dx * t;
            let py = a[1] + dy * t;
            let tx = dx / seg_len;
            let ty = dy / seg_len;
            return Some(([px, py], [tx, ty]));
        }
        acc += seg_len;
    }
    let a = points[points.len() - 2];
    let b = points[points.len() - 1];
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let seg_len = dx.hypot(dy).max(1e-12);
    Some(([b[0], b[1]], [dx / seg_len, dy / seg_len]))
}

/// Upright screen CCW angle from unit tangent (spec L4).
#[must_use]
pub fn upright_angle_deg(tangent: [f64; 2]) -> f64 {
    let mut deg = tangent[1].atan2(tangent[0]).to_degrees();
    if deg.abs() > 90.0 {
        deg += 180.0;
    }
    if deg > 180.0 {
        deg -= 360.0;
    }
    if deg <= -180.0 {
        deg += 360.0;
    }
    deg
}

/// Placement fractions for a segment length.
#[must_use]
pub fn placement_fractions(length_m: f64) -> Vec<f64> {
    if length_m > ROAD_NAME_LONG_SEGMENT_M {
        vec![0.25, 0.5, 0.75]
    } else {
        vec![0.5]
    }
}

/// Perpendicular distance from `(px,py)` to polyline (m).
#[must_use]
pub fn perpendicular_dist_to_polyline(points: &[[f64; 2]], px: f64, py: f64) -> f64 {
    if points.len() < 2 {
        return f64::INFINITY;
    }
    let mut best = f64::INFINITY;
    for w in points.windows(2) {
        let ax = w[0][0];
        let ay = w[0][1];
        let bx = w[1][0];
        let by = w[1][1];
        let dx = bx - ax;
        let dy = by - ay;
        let len_sq = dx * dx + dy * dy;
        if len_sq < 1e-18 {
            let d = (px - ax).hypot(py - ay);
            if d < best {
                best = d;
            }
            continue;
        }
        let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        let cx = ax + dx * t;
        let cy = ay + dy * t;
        let d = (px - cx).hypot(py - cy);
        if d < best {
            best = d;
        }
    }
    best
}

/// Dist m.
pub(crate) fn dist_m(a: &RoadLabelPlacement, b: &RoadLabelPlacement) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
