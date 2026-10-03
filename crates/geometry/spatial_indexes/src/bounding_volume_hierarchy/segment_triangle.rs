//! Where a segment crosses a triangle.
//!
//! **Role:** the both-sided Möller–Trumbore test of a segment against one triangle
//! ([`segment_hits_tri`]), the leaf test of every triangle tree query.
//! **Position:** under `bounding_volume_hierarchy`; read by
//! [`crate::bounding_volume_hierarchy::triangle_tree`] and by the line-of-sight tests that check a
//! traversal against brute force.
//! **Signals & state:** none; a pure function.
//! **Invariants:** winding is ignored; a segment within `DET_EPS` of the triangle's plane
//! misses; the barycentric bounds are widened by `BARY_EPS` so a crossing on a shared edge is
//! found by at least one of its triangles; the returned `t` is unclamped, its range check is the
//! caller's.

use geometry_primitives::vector3::{cross, dot, sub};

/// Determinant below which the segment counts as parallel to the triangle's plane.
pub(crate) const DET_EPS: f64 = 1e-12;

/// Slack on the barycentric bounds, so a crossing on an edge is not lost to rounding.
pub(crate) const BARY_EPS: f64 = 1e-9;

/// Both-sided Möller–Trumbore for segment p→q against triangle (a, b, c). Winding is ignored.
/// Returns the raw segment parameter t; the caller applies its own `[t_lo, t_hi]` range check
/// (traversal, tests and diagnostics each own their range).
#[must_use]
pub fn segment_hits_tri(
    p: [f64; 3],
    q: [f64; 3],
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
) -> Option<f64> {
    let dir = sub(q, p);
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let pvec = cross(dir, e2);
    let det = dot(e1, pvec);
    if det.abs() < DET_EPS {
        return None;
    }
    let inv = 1.0 / det;
    let tvec = sub(p, a);
    let u = dot(tvec, pvec) * inv;
    if !(-BARY_EPS..=1.0 + BARY_EPS).contains(&u) {
        return None;
    }
    let qvec = cross(tvec, e1);
    let v = dot(dir, qvec) * inv;
    if v < -BARY_EPS || u + v > 1.0 + BARY_EPS {
        return None;
    }
    Some(dot(e2, qvec) * inv)
}
