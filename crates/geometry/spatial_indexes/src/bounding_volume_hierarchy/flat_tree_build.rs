//! The one build core of every flat box tree: the triangle BVH and the world's box tree.
//!
//! **Role:** builds a flat binary tree of padded axis-aligned boxes over a set of items given by
//! their bounds and centroids ([`build_flat_tree`]), stopping at the leaf size and depth a caller's
//! [`BuildLimits`] name; [`BvhNode`] is the node record every such tree stores.
//! **Position:** under `bounding_volume_hierarchy`; called by
//! [`crate::bounding_volume_hierarchy::triangle_tree::Bvh::build`] over a mesh's triangles
//! ([`BuildLimits::TRIANGLES`]) and by the map engine's world line of sight over its instance boxes
//! (its own limits); the sidecar format stores the triangle tree's nodes byte for byte.
//! **Signals & state:** none; the build owns its node and order vectors until it returns them.
//! **Invariants:** an internal node's children are adjacent (`left_first`, `left_first + 1`); the
//! leaves tile `order` exactly; every node box contains its items' boxes padded by
//! [`NODE_BOX_PAD`] and rounded to f32; the split is the midpoint of the longest centroid axis,
//! with a median split when the midpoint leaves one side empty, so every split makes progress; the
//! same input gives the same tree, bit for bit.

/// Padding added on every side of a node box before it is rounded to f32, so a crossing on an
/// item's face is never lost to the rounding.
pub const NODE_BOX_PAD: f64 = 1e-3;

/// Parse-time depth bound. The traversals walk with a fixed 64-slot stack (net +1 per level);
/// rejecting trees deeper than 60 at parse keeps a hostile file from overflowing it.
pub(crate) const MAX_PARSE_DEPTH: u32 = 60;

/// Where a build stops splitting: a node with at most `leaf_max` items, or at depth `max_depth`,
/// becomes a leaf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildLimits {
    /// The most items a node holds before the build splits it.
    pub leaf_max: usize,

    /// The depth at which a node becomes a leaf whatever it holds.
    pub max_depth: usize,
}

impl BuildLimits {
    /// The triangle BVH's limits: 8 triangles per leaf, depth 32. The committed sidecar files are
    /// built with these, so changing them changes every emitted file.
    pub const TRIANGLES: Self = Self {
        leaf_max: 8,
        max_depth: 32,
    };
}

/// Flat tree node, 32 bytes, Wald layout: an internal node's children are adjacent (`left_first`
/// and `left_first + 1`); a leaf covers `order[left_first .. left_first + count]`. This layout is
/// the sidecar's node record; the size assertion below is the format's stride guarantee.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BvhNode {
    /// Lower corner of the padded node box.
    pub min: [f32; 3],

    /// Upper corner of the padded node box.
    pub max: [f32; 3],

    /// First child index (internal node) or first `order` slot (leaf).
    pub left_first: u32,

    /// 0 = internal node, > 0 = leaf item count.
    pub count: u32,
}

const _: () = assert!(std::mem::size_of::<BvhNode>() == 32);

impl BvhNode {
    const PLACEHOLDER: Self = Self {
        min: [0.0; 3],
        max: [0.0; 3],
        left_first: 0,
        count: 0,
    };
}

/// The bounds of one item a tree is built over: its box and the centroid the split sorts by.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemBounds {
    /// Lower corner of the item's box.
    pub lo: [f64; 3],

    /// Upper corner of the item's box.
    pub hi: [f64; 3],

    /// The point the split partitions the item by.
    pub centroid: [f64; 3],
}

impl ItemBounds {
    /// An item that is the box `lo..hi`, its centroid the box centre.
    #[must_use]
    pub fn of_box(lo: [f64; 3], hi: [f64; 3]) -> Self {
        Self {
            lo,
            hi,
            centroid: [
                0.5 * (lo[0] + hi[0]),
                0.5 * (lo[1] + hi[1]),
                0.5 * (lo[2] + hi[2]),
            ],
        }
    }
}

/// A built flat tree: its nodes, root first, and the item order its leaves cover.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FlatTree {
    /// The nodes, root at index 0; empty when the build had no item.
    pub nodes: Vec<BvhNode>,

    /// Item indices in leaf order.
    pub order: Vec<u32>,
}

/// Build a flat tree over the items `order` names (indices into `items`; an item whose index is
/// not in `order` is never read), within `limits`. An empty `order` gives an empty tree.
#[must_use]
pub fn build_flat_tree(items: &[ItemBounds], order: Vec<u32>, limits: BuildLimits) -> FlatTree {
    if order.is_empty() {
        return FlatTree::default();
    }
    let len = order.len();
    let mut builder = Builder {
        nodes: Vec::with_capacity(2 * len),
        order,
        items,
        limits,
    };
    builder.nodes.push(BvhNode::PLACEHOLDER);
    builder.build_into(0, 0, len, 0);
    FlatTree {
        nodes: builder.nodes,
        order: builder.order,
    }
}

struct Builder<'a> {
    nodes: Vec<BvhNode>,
    order: Vec<u32>,
    items: &'a [ItemBounds],
    limits: BuildLimits,
}

impl Builder<'_> {
    fn build_into(&mut self, node: usize, start: usize, end: usize, depth: usize) {
        let mut lo = [f64::MAX; 3];
        let mut hi = [f64::MIN; 3];
        let mut c_lo = [f64::MAX; 3];
        let mut c_hi = [f64::MIN; 3];
        for &t in &self.order[start..end] {
            let item = &self.items[t as usize];
            for a in 0..3 {
                lo[a] = lo[a].min(item.lo[a]);
                hi[a] = hi[a].max(item.hi[a]);
                c_lo[a] = c_lo[a].min(item.centroid[a]);
                c_hi[a] = c_hi[a].max(item.centroid[a]);
            }
        }

        let min = [
            (lo[0] - NODE_BOX_PAD) as f32,
            (lo[1] - NODE_BOX_PAD) as f32,
            (lo[2] - NODE_BOX_PAD) as f32,
        ];
        let max = [
            (hi[0] + NODE_BOX_PAD) as f32,
            (hi[1] + NODE_BOX_PAD) as f32,
            (hi[2] + NODE_BOX_PAD) as f32,
        ];
        let len = end - start;
        let extent = [c_hi[0] - c_lo[0], c_hi[1] - c_lo[1], c_hi[2] - c_lo[2]];
        let splittable = extent.iter().any(|&e| e > 0.0);
        if len <= self.limits.leaf_max || depth >= self.limits.max_depth || !splittable {
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
            .unwrap_or(0);
        let mid = 0.5 * (c_lo[axis] + c_hi[axis]);
        let mut i = start;
        let mut j = end;
        while i < j {
            if self.items[self.order[i] as usize].centroid[axis] < mid {
                i += 1;
            } else {
                j -= 1;
                self.order.swap(i, j);
            }
        }
        let mut split = i;
        if split == start || split == end {
            let items = self.items;
            self.order[start..end].sort_unstable_by(|&a, &b| {
                items[a as usize].centroid[axis].total_cmp(&items[b as usize].centroid[axis])
            });
            split = start + len / 2;
        }
        let l = self.nodes.len();
        self.nodes.push(BvhNode::PLACEHOLDER);
        self.nodes.push(BvhNode::PLACEHOLDER);
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

#[cfg(test)]
#[path = "tests/flat_tree_build_tests.rs"]
mod tests;
