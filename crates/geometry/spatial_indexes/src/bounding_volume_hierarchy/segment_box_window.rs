//! The slab test: where a segment runs inside an axis-aligned box.
//!
//! **Role:** clips a segment's parameter range to an axis-aligned box, the test every box-tree
//! walk (the world box tree, the interior instance walk) runs before it looks inside a box.
//! **Position:** under `bounding_volume_hierarchy`; read by the map engine's world line of sight
//! and its interior walker.
//! **Signals & state:** none; a pure function over its arguments.
//! **Invariants:** the window lies within `[0, 1]`, endpoints inclusive; a segment parallel to an
//! axis and outside that axis's slab misses; a miss is `None`, never an empty window.

/// Parametric window `[t_in, t_out]` of the segment `a→b` inside the box (`None` when it misses); the slab test, endpoints inclusive.
#[must_use]
pub fn segment_aabb_window(
    a: [f64; 3],
    b: [f64; 3],
    lo: [f64; 3],
    hi: [f64; 3],
) -> Option<(f64, f64)> {
    let mut t0 = 0.0f64;
    let mut t1 = 1.0f64;
    for k in 0..3 {
        let d = b[k] - a[k];
        if d.abs() < 1e-15 {
            if a[k] < lo[k] || a[k] > hi[k] {
                return None;
            }
            continue;
        }
        let mut ta = (lo[k] - a[k]) / d;
        let mut tb = (hi[k] - a[k]) / d;
        if ta > tb {
            core::mem::swap(&mut ta, &mut tb);
        }
        t0 = t0.max(ta);
        t1 = t1.min(tb);
        if t0 > t1 {
            return None;
        }
    }
    Some((t0, t1))
}
