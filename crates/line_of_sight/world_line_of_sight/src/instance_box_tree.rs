//! The world's box tree: every placed instance box a sight line may cross.
//!
//! **Role:** [`AabbTlas`] is a flat box tree over the instance boxes of the resident world, built by
//! the shared flat-tree core of `spatial_indexes` with this tree's limits, and lists the boxes a
//! segment crosses ([`AabbTlas::candidates`]) with their entry and exit parameters.
//! **Position:** built by [`crate::chunk_occluder`] over one resident chunk's row boxes and walked
//! by [`crate::raycast`] before it traces each crossed row's own meshes.
//! **Signals & state:** none; a built tree is immutable and owns its boxes.
//! **Invariants:** a box with `min > max` on an axis or a non-finite bound is absent: kept at its
//! index, never a candidate; `candidates` equals the brute-force `candidates_linear` and is sorted
//! by entry `t`, then index.

use spatial_indexes::bounding_volume_hierarchy::flat_tree_build::{
    BuildLimits, BvhNode, ItemBounds, build_flat_tree,
};
use spatial_indexes::bounding_volume_hierarchy::segment_box_window::segment_aabb_window;

/// The world box tree's limits: 4 boxes per leaf, depth 48.
const WORLD_BOX_LIMITS: BuildLimits = BuildLimits {
    leaf_max: 4,
    max_depth: 48,
};

/// An AABB tree over indexed boxes.
#[derive(Clone, Debug, Default)]
pub struct AabbTlas {
    nodes: Vec<BvhNode>,
    order: Vec<u32>,
    boxes: Vec<([f64; 3], [f64; 3])>,
}

/// A crossed box: parametric entry `t` on the query segment (0 when the segment starts inside) and the box index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    /// Where the segment enters the box (0 when it starts inside).
    pub t_entry: f64,

    /// Where the segment leaves the box.
    pub t_exit: f64,

    /// The box's index in the tree's box list.
    pub index: u32,
}

fn valid(b: &([f64; 3], [f64; 3])) -> bool {
    (0..3).all(|a| b.0[a] <= b.1[a] && b.0[a].is_finite() && b.1[a].is_finite())
}

impl AabbTlas {
    /// Build over `boxes` (`(min, max)` per index). A box with `min > max` on any axis (or a non-finite bound) is an "absent" box: it is kept at its index but never crosses anything.
    #[must_use]
    pub fn build(boxes: &[([f64; 3], [f64; 3])]) -> Self {
        let live: Vec<u32> = (0..boxes.len())
            .filter(|&i| valid(&boxes[i]))
            .map(|i| i as u32)
            .collect();
        let items: Vec<ItemBounds> = boxes
            .iter()
            .map(|&(lo, hi)| ItemBounds::of_box(lo, hi))
            .collect();
        let tree = build_flat_tree(&items, live, WORLD_BOX_LIMITS);
        Self {
            nodes: tree.nodes,
            order: tree.order,
            boxes: boxes.to_vec(),
        }
    }

    /// Number of boxes the tree was built over (absent ones included).
    #[must_use]
    pub fn len(&self) -> usize {
        self.boxes.len()
    }

    /// Whether no box is live.
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
        self.nodes.len() * core::mem::size_of::<BvhNode>()
            + self.order.len() * 4
            + self.boxes.len() * core::mem::size_of::<([f64; 3], [f64; 3])>()
    }

    /// Every box the segment `a→b` crosses (endpoints inclusive), sorted by entry `t` then index, appended to `out`.
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
