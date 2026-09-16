//! Role: index.
//! Position: `world/architecture/section` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::collections::HashMap;

use crate::spatial::bvh::sidecar::BvhSidecar;

/// Tile edge in cells. 16×16 f32 = 1 KiB per allocated tile.
pub const HEIGHT_TILE: usize = 16;

#[inline]
fn y_overlaps(y_min: f64, y_max: f64, y_lo: f64, y_hi: f64) -> bool {
    y_max >= y_lo && y_min <= y_hi
}

struct YNode {
    y_min: f64,
    y_max: f64,
    left_first: u32,

    count: u32,
}

/// 1-D BVH over triangle y-extents (Wald layout, midpoint split on y-centroid).
pub struct YIntervalIndex {
    nodes: Vec<YNode>,
    tri_order: Vec<u32>,
}

struct TriY {
    lo: f64,
    hi: f64,
    centroid: f64,
}

impl YIntervalIndex {
    /// Build over `occl`'s triangles. Empty mesh → empty index.
    #[must_use]
    pub fn build(occl: &BvhSidecar) -> Self {
        if occl.tris.is_empty() {
            return Self {
                nodes: Vec::new(),
                tri_order: Vec::new(),
            };
        }
        let info: Vec<TriY> = occl
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
                TriY {
                    lo,
                    hi,
                    centroid: 0.5 * (lo + hi),
                }
            })
            .collect();
        let mut nodes = Vec::with_capacity(2 * info.len());
        let mut tri_order: Vec<u32> = (0..info.len() as u32).collect();
        nodes.push(YNode {
            y_min: 0.0,
            y_max: 0.0,
            left_first: 0,
            count: 0,
        });
        build_into(&mut nodes, &mut tri_order, &info, 0, 0, info.len(), 0);
        Self { nodes, tri_order }
    }

    /// Triangle ids whose y-AABB overlaps closed `[y_lo, y_hi]`.
    #[must_use]
    pub fn query(&self, y_lo: f64, y_hi: f64) -> Vec<u32> {
        self.query_with_visits(y_lo, y_hi).0
    }

    /// Same as [`query`], plus the number of leaf triangles examined (the visit count).
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
            if !y_overlaps(n.y_min, n.y_max, y_lo, y_hi) {
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

fn build_into(
    nodes: &mut Vec<YNode>,
    tri_order: &mut [u32],
    info: &[TriY],
    node: usize,
    start: usize,
    end: usize,
    depth: u32,
) {
    let slice = &tri_order[start..end];
    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    for &ti in slice {
        let t = &info[ti as usize];
        y_min = y_min.min(t.lo);
        y_max = y_max.max(t.hi);
    }
    const LEAF: usize = 8;
    const MAX_DEPTH: u32 = 64;
    if end - start <= LEAF || depth >= MAX_DEPTH {
        nodes[node] = YNode {
            y_min,
            y_max,
            left_first: start as u32,
            count: (end - start) as u32,
        };
        return;
    }
    let mid = 0.5 * (y_min + y_max);
    let mut split = start;
    for i in start..end {
        let ti = tri_order[i];
        if info[ti as usize].centroid <= mid {
            tri_order.swap(i, split);
            split += 1;
        }
    }
    if split == start || split == end {
        split = start + (end - start) / 2;
    }
    let left = nodes.len();
    nodes.push(YNode {
        y_min: 0.0,
        y_max: 0.0,
        left_first: 0,
        count: 0,
    });
    nodes.push(YNode {
        y_min: 0.0,
        y_max: 0.0,
        left_first: 0,
        count: 0,
    });
    nodes[node] = YNode {
        y_min,
        y_max,
        left_first: left as u32,
        count: 0,
    };
    build_into(nodes, tri_order, info, left, start, split, depth + 1);
    build_into(nodes, tri_order, info, left + 1, split, end, depth + 1);
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
    /// New.
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

    /// Get.
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

    /// Set.
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
