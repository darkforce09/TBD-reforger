//! The triangle bounding volume hierarchy and its segment queries.
//!
//! **Role:** [`Bvh`] is a flat box tree over a mesh's triangles, built by the shared core
//! ([`crate::bounding_volume_hierarchy::flat_tree_build`]) with [`BuildLimits::TRIANGLES`], and
//! answers segment queries against it: any hit, nearest hit, every hit, each optionally restricted
//! to the [`SurfaceKind`]s a predicate accepts.
//! **Position:** under `bounding_volume_hierarchy`; built by the developer tools' BVH emitter and
//! parsed from sidecar files by [`crate::bounding_volume_hierarchy::sidecar`]; queried by the map
//! engine's building, interior and world line of sight.
//! **Signals & state:** none; a built tree is immutable and every query is a pure read.
//! **Invariants:** the traversal stack holds 64 entries, enough for every tree the build or the
//! sidecar parser admits; `first_hit(..).is_none()` exactly when `any_hit(..).is_none()` for the
//! same window and predicate; `all_hits` returns crossings sorted by `t`, then triangle.

use crate::bounding_volume_hierarchy::flat_tree_build::BuildLimits;
use crate::bounding_volume_hierarchy::flat_tree_build::BvhNode;
use crate::bounding_volume_hierarchy::flat_tree_build::ItemBounds;
use crate::bounding_volume_hierarchy::flat_tree_build::build_flat_tree;
use crate::bounding_volume_hierarchy::segment_triangle::segment_hits_tri;
use crate::bounding_volume_hierarchy::surface_kind::SurfaceKind;
use crate::bounding_volume_hierarchy::surface_kind::kind_of;
use geometry_primitives::vector3::sub;

/// A triangle bounding volume hierarchy: flat nodes and the triangle order its leaves cover. The
/// mesh it was built over is passed to every query.
#[derive(Debug)]
pub struct Bvh {
    /// Nodes, root first.
    pub(crate) nodes: Vec<BvhNode>,

    /// Triangle indices in leaf order.
    pub(crate) tri_order: Vec<u32>,
}

/// One crossing of a query segment with a triangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// Segment parameter of the crossing, 0 at the start and 1 at the end.
    pub t: f64,

    /// Index of the crossed triangle in the mesh's triangle table.
    pub tri: u32,
}

/// What a traversal visitor asks the walk to do after a crossing.
pub(crate) enum Flow {
    /// Keep walking with the window unchanged (collect-everything / skipped non-terminal).
    Continue,

    /// Accept this crossing as the new far bound — closest-hit search.
    ShrinkTo(f64),

    /// Done: the visitor has what it needs (any-hit).
    Stop,
}

impl Bvh {
    /// Build over a mesh with the shared flat-tree core and [`BuildLimits::TRIANGLES`]: midpoint
    /// split on the longest centroid axis, median split when the midpoint partition degenerates.
    /// Panics on an empty mesh — callers bail first.
    #[must_use]
    pub fn build(verts: &[[f64; 3]], tris: &[[u32; 3]]) -> Bvh {
        assert!(!tris.is_empty(), "Bvh::build on empty mesh");
        let items: Vec<ItemBounds> = tris
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
                ItemBounds::of_box(lo, hi)
            })
            .collect();
        let tree = build_flat_tree(
            &items,
            (0..tris.len() as u32).collect(),
            BuildLimits::TRIANGLES,
        );
        Bvh {
            nodes: tree.nodes,
            tri_order: tree.order,
        }
    }
}

impl Bvh {
    /// Number of nodes in the tree, leaves and internal nodes together.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Bvh {
    /// The root node's box as `(min, max)` corners widened to f64, in the mesh's frame; `None`
    /// for a tree with no nodes.
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
    /// The slab walk every query shares: visits each in-window crossing, in no particular order, and
    /// lets `visit` continue, shrink the far bound or stop.
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
    /// Every crossing in `[t_lo, t_hi]`, appended to `out` sorted by `t`, then triangle.
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

#[cfg(test)]
#[path = "tests/triangle_tree_tests.rs"]
mod tests;
