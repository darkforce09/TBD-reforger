//! Role: geometry.
//! Position: `world/architecture/blueprint` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Point at.
pub(crate) fn point_at(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [
        a[0] + t * (b[0] - a[0]),
        a[1] + t * (b[1] - a[1]),
        a[2] + t * (b[2] - a[2]),
    ]
}

/// 2D Euclidean distance helper.
#[must_use]
pub fn dist_2d(p1: [f64; 2], p2: [f64; 2]) -> f64 {
    ((p1[0] - p2[0]).powi(2) + (p1[1] - p2[1]).powi(2)).sqrt()
}

/// Point segment dist 2d.
pub(crate) fn point_segment_dist_2d(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> (f64, [f64; 2]) {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    let u = if len2 > 0.0 {
        (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let closest = [a[0] + u * ab[0], a[1] + u * ab[1]];
    (dist_2d(p, closest), closest)
}

/// Aabb contains 2d.
pub(crate) fn aabb_contains_2d(p: [f64; 2], min: [f64; 2], max: [f64; 2]) -> bool {
    p[0] >= min[0] && p[0] <= max[0] && p[1] >= min[1] && p[1] <= max[1]
}

/// Segment intersection t 2d.
pub(crate) fn segment_intersection_t_2d(
    p1: [f64; 2],
    p2: [f64; 2],
    q1: [f64; 2],
    q2: [f64; 2],
) -> Option<(f64, [f64; 2])> {
    let dx1 = p2[0] - p1[0];
    let dy1 = p2[1] - p1[1];
    let dx2 = q2[0] - q1[0];
    let dy2 = q2[1] - q1[1];

    let det = dx1 * dy2 - dy1 * dx2;
    if det.abs() < 1e-9 {
        return None;
    }

    let t = ((q1[0] - p1[0]) * dy2 - (q1[1] - p1[1]) * dx2) / det;
    let u = ((q1[0] - p1[0]) * dy1 - (q1[1] - p1[1]) * dx1) / det;

    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some((t, [p1[0] + t * dx1, p1[1] + t * dy1]))
    } else {
        None
    }
}

/// Calculates intersection point of two 2D line segments `(p1, p2)` and `(q1, q2)`.
#[must_use]
pub fn line_segment_intersection_2d(
    p1: [f64; 2],
    p2: [f64; 2],
    q1: [f64; 2],
    q2: [f64; 2],
) -> Option<[f64; 2]> {
    segment_intersection_t_2d(p1, p2, q1, q2).map(|(_, pt)| pt)
}

/// Segment aabb entry t 2d.
pub(crate) fn segment_aabb_entry_t_2d(
    p1: [f64; 2],
    p2: [f64; 2],
    min: [f64; 2],
    max: [f64; 2],
) -> Option<f64> {
    if aabb_contains_2d(p1, min, max) {
        return Some(0.0);
    }
    let corners = [
        [min[0], min[1]],
        [max[0], min[1]],
        [max[0], max[1]],
        [min[0], max[1]],
    ];
    let mut best: Option<f64> = None;
    for i in 0..4 {
        if let Some((t, _)) = segment_intersection_t_2d(p1, p2, corners[i], corners[(i + 1) % 4]) {
            best = Some(best.map_or(t, |b: f64| b.min(t)));
        }
    }
    best
}

/// Tests if 2D line segment intersects 2D axis-aligned bounding box.
#[must_use]
pub fn segment_intersects_aabb_2d(
    p1: [f64; 2],
    p2: [f64; 2],
    min: [f64; 2],
    max: [f64; 2],
) -> bool {
    segment_aabb_entry_t_2d(p1, p2, min, max).is_some()
}
