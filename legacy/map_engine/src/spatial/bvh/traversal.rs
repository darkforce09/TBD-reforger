//! Role: traversal.
//! Position: `spatial/bvh` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::bvh::node::Builder;
use crate::spatial::bvh::node::BvhNode;
use crate::spatial::bvh::node::TriInfo;
use crate::spatial::bvh::node::segment_hits_tri;
use crate::spatial::bvh::node::sub;
use crate::spatial::bvh::surface::SurfaceKind;
use crate::spatial::bvh::surface::kind_of;

const _: () = assert!(std::mem::size_of::<BvhNode>() == 32);

/// Bvh.
#[derive(Debug)]
pub struct Bvh {
    /// Nodes.
    pub(crate) nodes: Vec<BvhNode>,

    /// Tri order.
    pub(crate) tri_order: Vec<u32>,
}

/// Hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// T.
    pub t: f64,

    /// Tri.
    pub tri: u32,
}

/// Flow.
pub(crate) enum Flow {
    /// Keep walking with the window unchanged (collect-everything / skipped non-terminal).
    Continue,

    /// Accept this crossing as the new far bound — closest-hit search.
    ShrinkTo(f64),

    /// Done: the visitor has what it needs (any-hit).
    Stop,
}

impl Bvh {
    /// Midpoint split on the longest centroid axis, median-split fallback when the midpoint partition degenerates (guarantees strict progress; `MAX_DEPTH` is belt-and-suspenders). Panics on an empty mesh — callers bail first.
    pub fn build(verts: &[[f64; 3]], tris: &[[u32; 3]]) -> Bvh {
        assert!(!tris.is_empty(), "Bvh::build on empty mesh");
        let info: Vec<TriInfo> = tris
            .iter()
            .map(|t| {
                let mut lo = [f64::MAX; 3];
                let mut hi = [f64::MIN; 3];
                for &i in t {
                    let v = verts[i as usize];
                    for a in 0..3 {
                        lo[a] = lo[a].min(v[a]);
                        hi[a] = hi[a].max(v[a]);
                    }
                }
                let centroid = [
                    0.5 * (lo[0] + hi[0]),
                    0.5 * (lo[1] + hi[1]),
                    0.5 * (lo[2] + hi[2]),
                ];
                TriInfo { lo, hi, centroid }
            })
            .collect();
        let mut b = Builder {
            nodes: Vec::with_capacity(2 * tris.len()),
            tri_order: (0..tris.len() as u32).collect(),
            info: &info,
        };
        b.nodes.push(BvhNode {
            min: [0.0; 3],
            max: [0.0; 3],
            left_first: 0,
            count: 0,
        });
        b.build_into(0, 0, tris.len(), 0);
        Bvh {
            nodes: b.nodes,
            tri_order: b.tri_order,
        }
    }
}

impl Bvh {
    /// Node count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Bvh {
    /// Root bounds.
    #[must_use]
    pub fn root_bounds(&self) -> Option<([f64; 3], [f64; 3])> {
        let n = self.nodes.first()?;
        Some((
            [
                f64::from(n.min[0]),
                f64::from(n.min[1]),
                f64::from(n.min[2]),
            ],
            [
                f64::from(n.max[0]),
                f64::from(n.max[1]),
                f64::from(n.max[2]),
            ],
        ))
    }
}

impl Bvh {
    /// Traverse.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn traverse<V: FnMut(Hit) -> Flow>(
        &self,
        verts: &[[f64; 3]],
        tris: &[[u32; 3]],
        p: [f64; 3],
        q: [f64; 3],
        t_lo: f64,
        t_hi: f64,
        mut visit: V,
    ) {
        let dir = sub(q, p);
        let inv = [1.0 / dir[0], 1.0 / dir[1], 1.0 / dir[2]];
        let mut t_far = t_hi;

        let mut stack = [0u32; 64];
        stack[0] = 0;
        let mut sp = 1usize;
        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            let mut lo = t_lo;
            let mut hi = t_far;
            for a in 0..3 {
                let t1 = (f64::from(node.min[a]) - p[a]) * inv[a];
                let t2 = (f64::from(node.max[a]) - p[a]) * inv[a];
                lo = lo.max(t1.min(t2));
                hi = hi.min(t1.max(t2));
            }
            if lo > hi {
                continue;
            }
            if node.count > 0 {
                for k in node.left_first..node.left_first + node.count {
                    let tri = self.tri_order[k as usize];
                    let [ia, ib, ic] = tris[tri as usize];
                    if let Some(t) = segment_hits_tri(
                        p,
                        q,
                        verts[ia as usize],
                        verts[ib as usize],
                        verts[ic as usize],
                    ) && t >= t_lo
                        && t <= t_far
                    {
                        match visit(Hit { t, tri }) {
                            Flow::Continue => {}
                            Flow::ShrinkTo(new_far) => t_far = new_far,
                            Flow::Stop => return,
                        }
                    }
                }
            } else {
                debug_assert!(sp + 2 <= stack.len());
                stack[sp] = node.left_first;
                stack[sp + 1] = node.left_first + 1;
                sp += 2;
            }
        }
    }
}

impl Bvh {
    /// Stack-based any-hit over segment p→q: accepts the FIRST triangle whose raw t lands in [t_lo, t_hi] — not the nearest; sufficient for occlusion and miss diagnostics. Every triangle counts (`any_hit_where` with `|_| true`).
    pub fn any_hit(
        &self,
        verts: &[[f64; 3]],
        tris: &[[u32; 3]],
        p: [f64; 3],
        q: [f64; 3],
        t_lo: f64,
        t_hi: f64,
    ) -> Option<Hit> {
        self.any_hit_where(verts, tris, &[], p, q, t_lo, t_hi, |_| true)
    }
}

impl Bvh {
    /// Any-hit restricted to the triangles `terminal` accepts: the first in-window crossing whose [`SurfaceKind`] (from `kinds`, `Opaque` when the table is shorter — the wrappers pass an empty one) satisfies the predicate. Glass and foliage crossings are skipped without ending the walk, so `any_hit_where(.., SurfaceKind::is_terminal)` is the "does anything solid stand between" question a viewshed asks.
    #[allow(clippy::too_many_arguments)]
    pub fn any_hit_where<F: Fn(SurfaceKind) -> bool>(
        &self,
        verts: &[[f64; 3]],
        tris: &[[u32; 3]],
        kinds: &[SurfaceKind],
        p: [f64; 3],
        q: [f64; 3],
        t_lo: f64,
        t_hi: f64,
        terminal: F,
    ) -> Option<Hit> {
        let mut found = None;
        self.traverse(verts, tris, p, q, t_lo, t_hi, |h| {
            if terminal(kind_of(kinds, h.tri)) {
                found = Some(h);
                Flow::Stop
            } else {
                Flow::Continue
            }
        });
        found
    }
}

impl Bvh {
    /// Closest-hit traversal over segment p→q: same slab walk as [`Bvh::any_hit`] but tracks the best t and shrinks the range instead of returning on first acceptance — `first_hit(..).is_none()` ⇔ `any_hit(..).is_none()` by construction (existence is decided by the same [t_lo, t_hi] test; only the returned t/tri differ).
    pub fn first_hit(
        &self,
        verts: &[[f64; 3]],
        tris: &[[u32; 3]],
        p: [f64; 3],
        q: [f64; 3],
        t_lo: f64,
        t_hi: f64,
    ) -> Option<Hit> {
        self.first_hit_where(verts, tris, &[], p, q, t_lo, t_hi, |_| true)
    }
}

impl Bvh {
    /// Closest hit among the triangles `terminal` accepts (see [`Bvh::any_hit_where`]). `first_hit_where(..).is_none()` ⇔ `any_hit_where(..).is_none()` for the same predicate.
    #[allow(clippy::too_many_arguments)]
    pub fn first_hit_where<F: Fn(SurfaceKind) -> bool>(
        &self,
        verts: &[[f64; 3]],
        tris: &[[u32; 3]],
        kinds: &[SurfaceKind],
        p: [f64; 3],
        q: [f64; 3],
        t_lo: f64,
        t_hi: f64,
        terminal: F,
    ) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        self.traverse(verts, tris, p, q, t_lo, t_hi, |h| {
            if terminal(kind_of(kinds, h.tri)) {
                best = Some(h);
                Flow::ShrinkTo(h.t)
            } else {
                Flow::Continue
            }
        });
        best
    }
}

impl Bvh {
    /// All hits.
    #[allow(clippy::too_many_arguments)]
    pub fn all_hits(
        &self,
        verts: &[[f64; 3]],
        tris: &[[u32; 3]],
        p: [f64; 3],
        q: [f64; 3],
        t_lo: f64,
        t_hi: f64,
        out: &mut Vec<Hit>,
    ) {
        let start = out.len();
        self.traverse(verts, tris, p, q, t_lo, t_hi, |h| {
            out.push(h);
            Flow::Continue
        });
        out[start..].sort_by(|a, b| a.t.total_cmp(&b.t).then(a.tri.cmp(&b.tri)));
    }
}
