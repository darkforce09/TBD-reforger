//! The shared selection geometry: principal axis, convex hull, scatter seed.
//!
//! **Role:** the dominant axis of a point cloud, its convex hull and containment test, and the
//! order-independent seed the scatter draws from.
//! **Position:** re-exported at the crate root; the patterns and the align commands call it.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a degenerate covariance yields the due-east axis; the seed ignores id order.

use crate::patterns::{Pt, centroid};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// The dominant eigenvector (unit) of the 2×2 position covariance — the "principal axis" the Line and AlongLine commands run along. Returns `(ux, uy)`, a unit vector. Degenerate covariance (coincident/isotropic points) falls back to `(1, 0)` (due east) so downstream math never sees NaN.
#[must_use]
pub fn principal_axis(pts: &[Pt]) -> (f64, f64) {
    let n = pts.len();
    if n < 2 {
        return (1.0, 0.0);
    }
    let c = centroid(pts);
    let (mut a, mut b, mut d) = (0.0, 0.0, 0.0);
    for p in pts {
        let dx = p.x - c.x;
        let dy = p.y - c.y;
        a += dx * dx;
        b += dx * dy;
        d += dy * dy;
    }

    if b.abs() < 1e-12 {
        return if a >= d { (1.0, 0.0) } else { (0.0, 1.0) };
    }
    let trace = a + d;
    let det = a * d - b * b;
    let disc = (trace * trace / 4.0 - det).max(0.0).sqrt();
    let lambda = trace / 2.0 + disc;

    let (vx, vy) = (b, lambda - a);
    let len = (vx * vx + vy * vy).sqrt();
    if len < 1e-12 {
        (1.0, 0.0)
    } else {
        (vx / len, vy / len)
    }
}

/// Convex hull (counter-clockwise, no repeated endpoint) via Andrew's monotone chain. Returns the input's distinct points when `< 3` remain after the chain (a point or a line has no area). Used by [`pattern_fill_area`](crate::pattern_fill_area) for the scatter containment test.
#[must_use]
pub fn convex_hull(pts: &[Pt]) -> Vec<Pt> {
    let mut p: Vec<Pt> = pts.to_vec();
    p.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    p.dedup_by(|a, b| a.x == b.x && a.y == b.y);
    let n = p.len();
    if n < 3 {
        return p;
    }
    let cross = |o: Pt, a: Pt, b: Pt| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut hull: Vec<Pt> = Vec::with_capacity(2 * n);

    for &pt in &p {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], pt) <= 0.0 {
            hull.pop();
        }
        hull.push(pt);
    }

    let lower_len = hull.len() + 1;
    for &pt in p.iter().rev().skip(1) {
        while hull.len() >= lower_len
            && cross(hull[hull.len() - 2], hull[hull.len() - 1], pt) <= 0.0
        {
            hull.pop();
        }
        hull.push(pt);
    }
    hull.pop();
    hull
}

/// Is `pt` inside (or on the boundary of) the CCW convex `hull`? A point/line hull (`< 3`) can't contain area → `false`. Uses the sign of the cross product for every directed edge (all `>= 0` for a CCW hull means inside-or-on).
#[must_use]
pub fn point_in_convex_hull(hull: &[Pt], pt: Pt) -> bool {
    let n = hull.len();
    if n < 3 {
        return false;
    }
    for i in 0..n {
        let a = hull[i];
        let b = hull[(i + 1) % n];
        let cross = (b.x - a.x) * (pt.y - a.y) - (b.y - a.y) * (pt.x - a.x);
        if cross < 0.0 {
            return false;
        }
    }
    true
}

/// The scatter seed of a selection: the XOR of each id's `DefaultHasher` digest, so the same ids
/// give the same seed in any order. `DefaultHasher` is stable within one build of the toolchain.
#[must_use]
pub fn seed_from_ids(ids: &[String]) -> u64 {
    let mut acc: u64 = 0;
    for id in ids {
        let mut h = DefaultHasher::new();
        id.hash(&mut h);
        acc ^= h.finish();
    }
    acc
}
