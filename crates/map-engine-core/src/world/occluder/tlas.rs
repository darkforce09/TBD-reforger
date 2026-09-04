//! T-090.12.3 — the per-chunk TLAS: a bounding-volume tree over the placed instances' world
//! AABBs. A segment asks it for the boxes it crosses, in entry order, before any BLAS is
//! traced. It is NOT the triangle [`Bvh`](crate::bvh::Bvh): a triangle tree reports crossings
//! of faces, so an observer standing inside a box (under a canopy) would see no entry face and
//! the box would vanish — this tree tests the boxes themselves with the slab window, which is
//! `[0, t_out]` for a segment that starts inside.
//!
//! Layout follows the triangle tree: midpoint split on the longest centroid axis with a
//! median fallback, f32 node bounds padded outward, iterative traversal with a fixed stack.
//! ~64 B per box (two nodes per leaf pair) — a 4,000-row chunk is ~260 KB.

use crate::building_compound_los::segment_aabb_window;

/// Boxes per leaf.
const LEAF_MAX: usize = 4;
const MAX_DEPTH: u32 = 48;
/// Outward padding of the f32 node bounds (the triangle tree's `AABB_PAD`).
const PAD: f32 = 1e-3;

#[derive(Clone, Copy, Debug)]
struct Node {
    min: [f32; 3],
    max: [f32; 3],
    /// Internal: index of the left child (right = left + 1). Leaf: first index into `order`.
    left_first: u32,
    /// 0 = internal, > 0 = leaf box count.
    count: u32,
}

/// An AABB tree over indexed boxes.
#[derive(Clone, Debug, Default)]
pub struct AabbTlas {
    nodes: Vec<Node>,
    order: Vec<u32>,
    boxes: Vec<([f64; 3], [f64; 3])>,
}

/// A crossed box: parametric entry `t` on the query segment (0 when the segment starts
/// inside) and the box index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    pub t_entry: f64,
    pub t_exit: f64,
    pub index: u32,
}

fn valid(b: &([f64; 3], [f64; 3])) -> bool {
    (0..3).all(|a| b.0[a] <= b.1[a] && b.0[a].is_finite() && b.1[a].is_finite())
}

impl AabbTlas {
    /// Build over `boxes` (`(min, max)` per index). A box with `min > max` on any axis (or a
    /// non-finite bound) is an "absent" box: it is kept at its index but never crosses anything.
    #[must_use]
    pub fn build(boxes: &[([f64; 3], [f64; 3])]) -> Self {
        let live: Vec<u32> = (0..boxes.len())
            .filter(|&i| valid(&boxes[i]))
            .map(|i| i as u32)
            .collect();
        let mut t = Self {
            nodes: Vec::new(),
            order: live,
            boxes: boxes.to_vec(),
        };
        if t.order.is_empty() {
            return t;
        }
        t.nodes.reserve(2 * t.order.len());
        t.nodes.push(Node {
            min: [0.0; 3],
            max: [0.0; 3],
            left_first: 0,
            count: 0,
        });
        let n = t.order.len();
        t.build_into(0, 0, n, 0);
        t
    }

    fn centroid(&self, i: u32) -> [f64; 3] {
        let (lo, hi) = self.boxes[i as usize];
        [
            0.5 * (lo[0] + hi[0]),
            0.5 * (lo[1] + hi[1]),
            0.5 * (lo[2] + hi[2]),
        ]
    }

    fn bounds_of(&self, start: usize, end: usize) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for &i in &self.order[start..end] {
            let (a, b) = self.boxes[i as usize];
            for k in 0..3 {
                lo[k] = lo[k].min(a[k]);
                hi[k] = hi[k].max(b[k]);
            }
        }
        (
            [lo[0] as f32 - PAD, lo[1] as f32 - PAD, lo[2] as f32 - PAD],
            [hi[0] as f32 + PAD, hi[1] as f32 + PAD, hi[2] as f32 + PAD],
        )
    }

    fn build_into(&mut self, node: usize, start: usize, end: usize, depth: u32) {
        let (min, max) = self.bounds_of(start, end);
        self.nodes[node].min = min;
        self.nodes[node].max = max;
        let count = end - start;
        if count <= LEAF_MAX || depth >= MAX_DEPTH {
            self.nodes[node].left_first = start as u32;
            self.nodes[node].count = count as u32;
            return;
        }
        // Longest centroid axis; midpoint partition; median fallback when degenerate.
        let mut clo = [f64::INFINITY; 3];
        let mut chi = [f64::NEG_INFINITY; 3];
        for &i in &self.order[start..end] {
            let c = self.centroid(i);
            for k in 0..3 {
                clo[k] = clo[k].min(c[k]);
                chi[k] = chi[k].max(c[k]);
            }
        }
        let mut axis = 0;
        for k in 1..3 {
            if chi[k] - clo[k] > chi[axis] - clo[axis] {
                axis = k;
            }
        }
        let mid = 0.5 * (clo[axis] + chi[axis]);
        let slice = &mut self.order[start..end];
        let mut split = 0usize;
        {
            let boxes = &self.boxes;
            let cen = |i: u32| {
                let (lo, hi) = boxes[i as usize];
                0.5 * (lo[axis] + hi[axis])
            };
            for j in 0..slice.len() {
                if cen(slice[j]) < mid {
                    slice.swap(j, split);
                    split += 1;
                }
            }
            if split == 0 || split == slice.len() {
                slice.sort_by(|&a, &b| cen(a).total_cmp(&cen(b)).then(a.cmp(&b)));
                split = slice.len() / 2;
            }
        }
        let left = self.nodes.len();
        self.nodes.push(Node {
            min: [0.0; 3],
            max: [0.0; 3],
            left_first: 0,
            count: 0,
        });
        self.nodes.push(Node {
            min: [0.0; 3],
            max: [0.0; 3],
            left_first: 0,
            count: 0,
        });
        self.nodes[node].left_first = left as u32;
        self.nodes[node].count = 0;
        self.build_into(left, start, start + split, depth + 1);
        self.build_into(left + 1, start + split, end, depth + 1);
    }

    /// Number of boxes the tree was built over (absent ones included).
    #[must_use]
    pub fn len(&self) -> usize {
        self.boxes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// The box at `index`.
    #[must_use]
    pub fn get(&self, index: u32) -> Option<([f64; 3], [f64; 3])> {
        self.boxes.get(index as usize).copied()
    }

    /// Root bounds (padded), `None` when no box is live.
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

    /// Heap bytes of the tree.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.nodes.len() * core::mem::size_of::<Node>()
            + self.order.len() * 4
            + self.boxes.len() * core::mem::size_of::<([f64; 3], [f64; 3])>()
    }

    /// Every box the segment `a→b` crosses (endpoints inclusive), sorted by entry `t` then
    /// index, appended to `out`.
    pub fn candidates(&self, a: [f64; 3], b: [f64; 3], out: &mut Vec<Candidate>) {
        if self.nodes.is_empty() {
            return;
        }
        let start = out.len();
        let mut stack: [u32; 64] = [0; 64];
        let mut sp = 0usize;
        stack[sp] = 0;
        sp += 1;
        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            let lo = [
                f64::from(node.min[0]),
                f64::from(node.min[1]),
                f64::from(node.min[2]),
            ];
            let hi = [
                f64::from(node.max[0]),
                f64::from(node.max[1]),
                f64::from(node.max[2]),
            ];
            if segment_aabb_window(a, b, lo, hi).is_none() {
                continue;
            }
            if node.count > 0 {
                let first = node.left_first as usize;
                for &i in &self.order[first..first + node.count as usize] {
                    let (blo, bhi) = self.boxes[i as usize];
                    if let Some((t0, t1)) = segment_aabb_window(a, b, blo, bhi) {
                        out.push(Candidate {
                            t_entry: t0,
                            t_exit: t1,
                            index: i,
                        });
                    }
                }
            } else if sp + 2 <= stack.len() {
                stack[sp] = node.left_first + 1;
                stack[sp + 1] = node.left_first;
                sp += 2;
            }
        }
        out[start..].sort_by(|x, y| x.t_entry.total_cmp(&y.t_entry).then(x.index.cmp(&y.index)));
    }

    /// Brute-force reference of [`Self::candidates`] (tests, and the linear fallback).
    pub fn candidates_linear(&self, a: [f64; 3], b: [f64; 3], out: &mut Vec<Candidate>) {
        let start = out.len();
        for (i, bx) in self.boxes.iter().enumerate() {
            if !valid(bx) {
                continue;
            }
            if let Some((t0, t1)) = segment_aabb_window(a, b, bx.0, bx.1) {
                out.push(Candidate {
                    t_entry: t0,
                    t_exit: t1,
                    index: i as u32,
                });
            }
        }
        out[start..].sort_by(|x, y| x.t_entry.total_cmp(&y.t_entry).then(x.index.cmp(&y.index)));
    }
}
