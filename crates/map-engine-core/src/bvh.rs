//! BVH any-hit raycaster over collision trimeshes + the binary `.bvh` sidecar codec —
//! the 3D occlusion lane (T-090.6) that retired the 2.5D interpretation model from LOS
//! duty. Moved here from `xtask/src/map_blueprint/bvh.rs` (step 1, verbatim) so the
//! sidecar loader, `evaluate_los` (step 3), and the viewshed pipeline (step 4) share one
//! implementation; xtask keeps only the CLI (`map bvh-parity` / `map bvh-emit`).
//!
//! The triangle test ignores winding (|det|): any-hit occlusion is orientation-agnostic,
//! which sidesteps the COLL winding-inversion trap entirely.
//!
//! # Surface kinds (T-090.11.1)
//!
//! Every triangle carries a [`SurfaceKind`] — `Opaque` (terminal: walls, door leaves, trunks),
//! `Glass` (visual LOS continues) or `Foliage` (soft cover, attenuates with depth). The
//! traversals come in a filtered form ([`Bvh::any_hit_where`] / [`Bvh::first_hit_where`]) that
//! judges only the triangles a predicate calls terminal, and a multi-hit form
//! ([`Bvh::all_hits`]) that returns every crossing so a caller can walk glass and foliage
//! kind by kind. [`Bvh::any_hit`] / [`Bvh::first_hit`] are the `|_| true` wrappers — their
//! results are bit-identical to the pre-v2 traversals (the 400/400 parity pins).
//!
//! # Sidecar format v2 (`<slug>.bvh`, little-endian throughout)
//!
//! | Section | Bytes |
//! |---|---|
//! | header | 32: magic `b"TBVH"` · version u32 = 2 · nverts u32 · ntris u32 · nnodes u32 · flags u32 · reserved u32×2 = 0 |
//! | verts | nverts × 3 × f32 |
//! | tris | ntris × 3 × u32 |
//! | nodes | nnodes × 32 (min f32×3 · max f32×3 · left_first u32 · count u32) |
//! | tri_order | ntris × u32 |
//! | kinds | (flags bit 0 set) ntris × u8 [`SurfaceKind`] codes, zero-padded to a 4-byte boundary |
//!
//! Version 1 files (no flags, no kinds section) still parse: every triangle reads as
//! `Opaque`. The emitter always writes version 2 with the kinds section.
//!
//! Determinism contract: the emitter quantizes verts f64→f32 ([`quantize_verts`]), lifts
//! back ([`lift_verts`], exact), and builds the BVH over the lifted values — so raycasts
//! over a parsed sidecar are bit-identical to the emitter's, and
//! `emit(parse(emit(x))) == emit(x)`. The build itself uses only IEEE basic ops and
//! `total_cmp` sorts on a fixed input order — byte-identical output across runs and
//! targets. [`BvhSidecar::parse`] fully validates structure so [`Bvh::any_hit`] can
//! neither panic nor hang on arbitrary input bytes (sidecars are fetched over HTTP later).

/// Conservative pad on f64→f32 node-bound storage. AABBs only cull, so padding can only
/// cost extra node visits, never a false miss; at building scale (|coord| ≤ ~30 m) the
/// f32 cast error is ≤ ~4e-6 m, three orders under the pad.
const AABB_PAD: f64 = 1e-3;
const LEAF_MAX: usize = 8;
const MAX_DEPTH: usize = 32;
/// |det| below this = segment parallel to the triangle plane (or degenerate triangle) —
/// same threshold as the voxel marcher's Möller–Trumbore in xtask's `mesh.rs`.
const DET_EPS: f64 = 1e-12;
/// Barycentric slack (xtask `mesh.rs` precedent): closes shared-edge cracks between
/// 0.01-quantized COLL verts without measurably expanding any triangle.
const BARY_EPS: f64 = 1e-9;

/// Parse-time depth bound. [`Bvh::any_hit`] walks with a fixed 64-slot stack (net +1 per
/// level); rejecting > 60 at parse keeps hostile-but-forward files from overflowing it.
const MAX_PARSE_DEPTH: u32 = 60;

/// Material class of one collision triangle (T-090.11.1). The wire code is the `u8`
/// discriminant — appending a variant is additive; renumbering is a format break.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SurfaceKind {
    /// Wood, stone, brick, metal, door leaves, tree trunks: terminates visual LOS and blocks
    /// movement.
    Opaque = 0,
    /// Window panes, glass doors, display cases: visual LOS continues (a small optical
    /// concealment), movement is blocked.
    Glass = 1,
    /// Tree canopies, bushes, soft vegetation: soft cover — concealment accumulates with the
    /// depth traversed inside the volume.
    Foliage = 2,
}

impl SurfaceKind {
    /// Highest wire code the codec accepts.
    pub const MAX_CODE: u8 = 2;

    /// Decode a wire code; `None` for anything this build does not know (the sidecar parser
    /// rejects such files rather than guessing).
    #[must_use]
    pub fn from_u8(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Opaque),
            1 => Some(Self::Glass),
            2 => Some(Self::Foliage),
            _ => None,
        }
    }

    /// The wire code.
    #[must_use]
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Does a visual line of sight stop here? Only `Opaque` is terminal — glass and foliage
    /// are annotations the multi-hit walk continues through.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Opaque)
    }
}

pub fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Both-sided Möller–Trumbore for segment p→q against triangle (a, b, c). Winding is
/// ignored. Returns the raw segment parameter t — the CALLER applies the [t_lo, t_hi]
/// range check (traversal, tests, and diagnostics each own their range).
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

/// Flat BVH node, 32 bytes, Wald layout: an internal node's children are adjacent
/// (`left_first` and `left_first + 1`); a leaf covers
/// `tri_order[left_first .. left_first + count]`. This layout IS the sidecar's node
/// record — the size assert below doubles as the format's stride guarantee.
#[derive(Debug, Clone, Copy)]
struct BvhNode {
    min: [f32; 3],
    max: [f32; 3],
    left_first: u32,
    /// 0 = internal node, > 0 = leaf triangle count.
    count: u32,
}
const _: () = assert!(std::mem::size_of::<BvhNode>() == 32);

/// Owns no geometry: `build` and the traversals take the same `verts`/`tris` slices, and the
/// `Hit::tri` index is the ORIGINAL triangle index (valid into `tris` and any parallel
/// per-triangle table the caller keeps — the sidecar's `kinds` is one).
#[derive(Debug)]
pub struct Bvh {
    nodes: Vec<BvhNode>,
    tri_order: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub t: f64,
    pub tri: u32,
}

struct TriInfo {
    lo: [f64; 3],
    hi: [f64; 3],
    centroid: [f64; 3],
}

struct Builder<'a> {
    nodes: Vec<BvhNode>,
    tri_order: Vec<u32>,
    info: &'a [TriInfo],
}

/// What a traversal visitor tells the walk after each triangle crossing inside the current
/// `[t_lo, t_hi]` window.
enum Flow {
    /// Keep walking with the window unchanged (collect-everything / skipped non-terminal).
    Continue,
    /// Accept this crossing as the new far bound — closest-hit search.
    ShrinkTo(f64),
    /// Done: the visitor has what it needs (any-hit).
    Stop,
}

impl Bvh {
    /// Midpoint split on the longest centroid axis, median-split fallback when the
    /// midpoint partition degenerates (guarantees strict progress; `MAX_DEPTH` is
    /// belt-and-suspenders). Panics on an empty mesh — callers bail first.
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

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Root bounds as stored (f32, padded by `AABB_PAD`) — the whole mesh's AABB, used to cull
    /// an instance before its ray is transformed. `None` only for the impossible empty tree.
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

    /// The one stack walk every public traversal is built on. Slab-culls nodes against the
    /// live `[t_lo, hi]` window, tests leaf triangles in `tri_order` order, and hands each
    /// in-window crossing to `visit`, which steers the walk ([`Flow`]). Slab math runs f64
    /// against the f32 bounds. When p sits exactly on a slab bound with dir 0 on that axis,
    /// `0.0 * inf` yields NaN; `f64::min`/`f64::max` return the non-NaN operand, so NaN
    /// degrades to "visit the node" — conservative, never a cull.
    #[allow(clippy::too_many_arguments)]
    fn traverse<V: FnMut(Hit) -> Flow>(
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
        // Stack depth ≤ MAX_DEPTH + 2: each internal pop nets +1 entry along one path.
        // Parsed sidecars are depth-bounded to MAX_PARSE_DEPTH for the same reason.
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

    /// Stack-based any-hit over segment p→q: accepts the FIRST triangle whose raw t lands
    /// in [t_lo, t_hi] — not the nearest; sufficient for occlusion and miss diagnostics.
    /// Every triangle counts (`any_hit_where` with `|_| true`).
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

    /// Any-hit restricted to the triangles `terminal` accepts: the first in-window crossing
    /// whose [`SurfaceKind`] (from `kinds`, `Opaque` when the table is shorter — the wrappers
    /// pass an empty one) satisfies the predicate. Glass and foliage crossings are skipped
    /// without ending the walk, so `any_hit_where(.., SurfaceKind::is_terminal)` is the
    /// "does anything solid stand between" question a viewshed asks.
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

    /// Closest-hit traversal over segment p→q: same slab walk as [`Bvh::any_hit`] but
    /// tracks the best t and shrinks the range instead of returning on first acceptance —
    /// `first_hit(..).is_none()` ⇔ `any_hit(..).is_none()` by construction (existence is
    /// decided by the same [t_lo, t_hi] test; only the returned t/tri differ).
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

    /// Closest hit among the triangles `terminal` accepts (see [`Bvh::any_hit_where`]).
    /// `first_hit_where(..).is_none()` ⇔ `any_hit_where(..).is_none()` for the same predicate.
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

    /// Every triangle crossing of segment p→q inside [t_lo, t_hi], appended to `out` in
    /// ascending `(t, tri)` order — the multi-hit primitive. A caller pairs the entry/exit
    /// crossings of a closed foliage volume, or steps past a glass pane, by walking this list
    /// with the sidecar's `kinds`. Coincident crossings (a shared edge on a quantized mesh) are
    /// all reported; the caller dedups by its own rule.
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

/// The kind of triangle `tri` in a possibly-short table: the wrappers pass `&[]`, so a
/// missing entry reads as `Opaque` (every triangle terminal — the pre-v2 semantics).
#[inline]
fn kind_of(kinds: &[SurfaceKind], tri: u32) -> SurfaceKind {
    kinds
        .get(tri as usize)
        .copied()
        .unwrap_or(SurfaceKind::Opaque)
}

impl Builder<'_> {
    fn build_into(&mut self, node: usize, start: usize, end: usize, depth: usize) {
        let mut lo = [f64::MAX; 3];
        let mut hi = [f64::MIN; 3];
        let mut c_lo = [f64::MAX; 3];
        let mut c_hi = [f64::MIN; 3];
        for &t in &self.tri_order[start..end] {
            let ti = &self.info[t as usize];
            for a in 0..3 {
                lo[a] = lo[a].min(ti.lo[a]);
                hi[a] = hi[a].max(ti.hi[a]);
                c_lo[a] = c_lo[a].min(ti.centroid[a]);
                c_hi[a] = c_hi[a].max(ti.centroid[a]);
            }
        }
        // Pad in f64 FIRST, then cast: the cast rounds to nearest, and the pad dwarfs it.
        let min = [
            (lo[0] - AABB_PAD) as f32,
            (lo[1] - AABB_PAD) as f32,
            (lo[2] - AABB_PAD) as f32,
        ];
        let max = [
            (hi[0] + AABB_PAD) as f32,
            (hi[1] + AABB_PAD) as f32,
            (hi[2] + AABB_PAD) as f32,
        ];
        let len = end - start;
        let extent = [c_hi[0] - c_lo[0], c_hi[1] - c_lo[1], c_hi[2] - c_lo[2]];
        let splittable = extent.iter().any(|&e| e > 0.0);
        if len <= LEAF_MAX || depth >= MAX_DEPTH || !splittable {
            self.nodes[node] = BvhNode {
                min,
                max,
                left_first: start as u32,
                count: len as u32,
            };
            return;
        }
        let axis = (0..3)
            .max_by(|&a, &b| extent[a].total_cmp(&extent[b]))
            .unwrap();
        let mid = 0.5 * (c_lo[axis] + c_hi[axis]);
        let mut i = start;
        let mut j = end;
        while i < j {
            if self.info[self.tri_order[i] as usize].centroid[axis] < mid {
                i += 1;
            } else {
                j -= 1;
                self.tri_order.swap(i, j);
            }
        }
        let mut split = i;
        if split == start || split == end {
            // Midpoint degenerated (all centroids one side): median split for progress.
            self.tri_order[start..end].sort_unstable_by(|&a, &b| {
                self.info[a as usize].centroid[axis]
                    .total_cmp(&self.info[b as usize].centroid[axis])
            });
            split = start + len / 2;
        }
        let l = self.nodes.len();
        let placeholder = BvhNode {
            min: [0.0; 3],
            max: [0.0; 3],
            left_first: 0,
            count: 0,
        };
        self.nodes.push(placeholder);
        self.nodes.push(placeholder);
        self.nodes[node] = BvhNode {
            min,
            max,
            left_first: l as u32,
            count: 0,
        };
        self.build_into(l, start, split, depth + 1);
        self.build_into(l + 1, split, end, depth + 1);
    }
}

/// The `.bvh` sidecar codec (parse / emit / version constants) — see the module doc above
/// for the format. Lives in `bvh_sidecar.rs` for the SIZE gate; re-exported here so the
/// public path stays `map_engine_core::bvh::*`.
#[path = "bvh_sidecar.rs"]
mod sidecar;
pub use sidecar::{
    BvhParseError, BvhSidecar, FLAG_KINDS, SIDECAR_MAGIC, SIDECAR_VERSION, SIDECAR_VERSION_MIN,
    emit_bytes, lift_verts, quantize_verts,
};

#[cfg(test)]
#[path = "bvh_tests.rs"]
pub(crate) mod tests;
