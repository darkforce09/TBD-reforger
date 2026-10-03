//! The y-interval index over a mesh's triangles and the sparse height raster.
//!
//! **Role:** finds the triangles whose y-extent overlaps a height interval through a flat box tree
//! built by `spatial_indexes`' [`build_flat_tree`] ([`YIntervalIndex`],
//! [`triangles_overlapping_y`]), checks the occlusion BVH's root encloses the mesh
//! ([`bvh_encloses_mesh`]), and stores heights in lazily allocated tiles ([`SparseHeights`]).
//! **Position:** read by [`crate::section::cutter`].
//! **Signals & state:** none; a [`SparseHeights`] owns its tiles.
//! **Invariants:** the candidates are a superset of the triangles that overlap the interval, so the
//! cutter's exact test decides; an unset or cleared cell reads as `None`.

use std::collections::HashMap;

use spatial_indexes::bounding_volume_hierarchy::flat_tree_build::{
    BuildLimits, BvhNode, ItemBounds, build_flat_tree,
};
use spatial_indexes::bounding_volume_hierarchy::sidecar::BvhSidecar;

/// Tile edge in cells. 16×16 f32 = 1 KiB per allocated tile.
pub const HEIGHT_TILE: usize = 16;

#[inline]
fn y_overlaps(y_min: f64, y_max: f64, y_lo: f64, y_hi: f64) -> bool {
    y_max >= y_lo && y_min <= y_hi
}

/// The y-interval index's build limits: 8 triangles per leaf, depth 64.
const Y_INTERVAL_LIMITS: BuildLimits = BuildLimits {
    leaf_max: 8,
    max_depth: 64,
};

/// 1-D BVH over triangle y-extents: the shared flat box tree built over each triangle's y-extent
/// alone (x and z collapsed to 0), so every split falls on the y-centroids.
pub struct YIntervalIndex {
    nodes: Vec<BvhNode>,
    tri_order: Vec<u32>,
}

impl YIntervalIndex {
    /// Build over `occl`'s triangles. Empty mesh → empty index.
    #[must_use]
    pub fn build(occl: &BvhSidecar) -> Self {
        let items: Vec<ItemBounds> = occl
            .tris
            .iter()
            .map(|t| {
                let mut lo = f64::MAX;
                let mut hi = f64::MIN;
                for &i in t {
                    let y = occl.verts[i as usize][1];
                    lo = lo.min(y);
                    hi = hi.max(y);
                }
                ItemBounds::of_box([0.0, lo, 0.0], [0.0, hi, 0.0])
            })
            .collect();
        let order: Vec<u32> = (0..items.len() as u32).collect();
        let tree = build_flat_tree(&items, order, Y_INTERVAL_LIMITS);
        Self {
            nodes: tree.nodes,
            tri_order: tree.order,
        }
    }

    /// Triangle ids whose y-AABB overlaps closed `[y_lo, y_hi]`, plus any other triangle of a
    /// leaf whose padded box overlaps it.
    #[must_use]
    pub fn query(&self, y_lo: f64, y_hi: f64) -> Vec<u32> {
        self.query_with_visits(y_lo, y_hi).0
    }

    /// Same as [`Self::query`], plus the number of leaf triangles examined (the visit count).
    #[must_use]
    pub fn query_with_visits(&self, y_lo: f64, y_hi: f64) -> (Vec<u32>, usize) {
        if y_hi < y_lo || self.nodes.is_empty() {
            return (Vec::new(), 0);
        }
        let mut out = Vec::new();
        let mut visits = 0usize;
        let mut stack = vec![0u32];
        while let Some(ni) = stack.pop() {
            let n = &self.nodes[ni as usize];
            if !y_overlaps(f64::from(n.min[1]), f64::from(n.max[1]), y_lo, y_hi) {
                continue;
            }
            if n.count == 0 {
                stack.push(n.left_first);
                stack.push(n.left_first + 1);
                continue;
            }
            let start = n.left_first as usize;
            let end = start + n.count as usize;
            for &ti in &self.tri_order[start..end] {
                visits += 1;
                out.push(ti);
            }
        }
        (out, visits)
    }
}

/// True when every vertex lies inside the occlusion BVH's root AABB (f32 bounds + pad).
#[must_use]
pub fn bvh_encloses_mesh(occl: &BvhSidecar) -> bool {
    let Some((lo, hi)) = occl.bvh.root_bounds() else {
        return occl.verts.is_empty();
    };
    const SLACK: f64 = 1e-3;
    occl.verts
        .iter()
        .all(|v| (0..3).all(|a| v[a] >= lo[a] - SLACK && v[a] <= hi[a] + SLACK))
}

/// Candidate triangle ids whose y-extent overlaps closed `[y_lo, y_hi]`.
#[must_use]
pub fn triangles_overlapping_y(occl: &BvhSidecar, y_lo: f64, y_hi: f64) -> Vec<u32> {
    triangles_overlapping_y_counted(occl, y_lo, y_hi).0
}

/// [`triangles_overlapping_y`] plus leaf-triangle visits (0 when the root slab misses).
#[must_use]
pub fn triangles_overlapping_y_counted(
    occl: &BvhSidecar,
    y_lo: f64,
    y_hi: f64,
) -> (Vec<u32>, usize) {
    if y_hi < y_lo || occl.tris.is_empty() {
        return (Vec::new(), 0);
    }
    if let Some((lo, hi)) = occl.bvh.root_bounds()
        && (hi[1] < y_lo || lo[1] > y_hi)
    {
        return (Vec::new(), 0);
    }
    YIntervalIndex::build(occl).query_with_visits(y_lo, y_hi)
}

/// Sparse tiled `f32` raster: NaN = no surface. Tiles allocate on first write.
#[derive(Clone, Debug)]
pub struct SparseHeights {
    cols: usize,
    rows: usize,
    tiles: HashMap<u32, Box<[f32]>>,
}

impl PartialEq for SparseHeights {
    fn eq(&self, other: &Self) -> bool {
        if self.cols != other.cols || self.rows != other.rows {
            return false;
        }
        if self.tiles.len() != other.tiles.len() {
            return false;
        }
        self.tiles.iter().all(|(k, a)| {
            other.tiles.get(k).is_some_and(|b| {
                a.len() == b.len()
                    && a.iter()
                        .zip(b.iter())
                        .all(|(x, y)| x.to_bits() == y.to_bits())
            })
        })
    }
}

impl SparseHeights {
    /// An empty `cols` by `rows` raster; no tile is allocated before the first write.
    #[must_use]
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            tiles: HashMap::new(),
        }
    }

    fn tile_cols(&self) -> usize {
        self.cols.div_ceil(HEIGHT_TILE).max(1)
    }

    fn key(tile_cols: usize, col: usize, row: usize) -> u32 {
        let tx = col / HEIGHT_TILE;
        let ty = row / HEIGHT_TILE;
        (ty * tile_cols + tx) as u32
    }

    /// Bytes of allocated tile payloads (no HashMap overhead). Empty field: 0.
    #[must_use]
    pub fn allocated_bytes(&self) -> usize {
        self.tiles.len() * HEIGHT_TILE * HEIGHT_TILE * std::mem::size_of::<f32>()
    }

    /// Height at `(col, row)` widened from `f32`; `None` out of bounds, in an unallocated
    /// tile or on a cleared cell.
    #[must_use]
    pub fn get(&self, col: usize, row: usize) -> Option<f64> {
        if col >= self.cols || row >= self.rows {
            return None;
        }
        let tile = self.tiles.get(&Self::key(self.tile_cols(), col, row))?;
        let lx = col % HEIGHT_TILE;
        let ly = row % HEIGHT_TILE;
        let v = tile[ly * HEIGHT_TILE + lx];
        if v.is_nan() { None } else { Some(f64::from(v)) }
    }

    /// Stores `y` as `f32` at `(col, row)`, allocating its tile; `None` clears the cell
    /// without allocating; a write out of bounds is ignored.
    pub fn set(&mut self, col: usize, row: usize, y: Option<f64>) {
        if col >= self.cols || row >= self.rows {
            return;
        }
        let tc = self.tile_cols();
        let key = Self::key(tc, col, row);
        let lx = col % HEIGHT_TILE;
        let ly = row % HEIGHT_TILE;
        let idx = ly * HEIGHT_TILE + lx;
        match y {
            None => {
                if let Some(tile) = self.tiles.get_mut(&key) {
                    tile[idx] = f32::NAN;
                }
            }
            Some(v) => {
                let tile = self.tiles.entry(key).or_insert_with(|| {
                    vec![f32::NAN; HEIGHT_TILE * HEIGHT_TILE].into_boxed_slice()
                });
                tile[idx] = v as f32;
            }
        }
    }

    /// Stored (non-NaN) heights in any tile order.
    pub fn iter_stored(&self) -> impl Iterator<Item = f64> + '_ {
        self.tiles.values().flat_map(|tile| {
            tile.iter()
                .filter_map(|&v| if v.is_nan() { None } else { Some(f64::from(v)) })
        })
    }
}

#[cfg(test)]
#[path = "tests/index_tests.rs"]
mod tests;
