//! T-938.4 — y-interval query over a sidecar's BVH, plus sparse HeightField tiles.
//!
//! [`triangles_overlapping_y`] walks a 1-D BVH built from the same triangles the occlusion
//! BVH indexes, after culling against [`crate::bvh::Bvh::root_bounds`]. That is the y-slab
//! analogue of the ray walk: candidate ids only, never the whole mesh, unless every triangle's
//! y-extent overlaps the query.
//!
//! [`SparseHeights`] stores `f32` cells with a NaN sentinel in tiles allocated on first write.

use std::collections::HashMap;

use crate::bvh::BvhSidecar;

/// Tile edge in cells. 16×16 f32 = 1 KiB per allocated tile.
pub const HEIGHT_TILE: usize = 16;

/// Closed `[y_lo, y_hi]` overlap against a node's y-AABB.
#[inline]
fn y_overlaps(y_min: f64, y_max: f64, y_lo: f64, y_hi: f64) -> bool {
    y_max >= y_lo && y_min <= y_hi
}

struct YNode {
    y_min: f64,
    y_max: f64,
    left_first: u32,
    /// 0 = internal (children at `left_first`, `left_first + 1`); >0 = leaf count.
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
    ///
    /// An inverted interval (`y_hi < y_lo`) is empty. A zero-height interval `[y, y]` is a
    /// valid plane query and returns every triangle that straddles `y`.
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
///
/// Culls against the existing BVH root y-slab first, then walks [`YIntervalIndex`].
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
    // Closed [y, y] is a valid plane query; only inverted (y_hi < y_lo) is empty.
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
mod tests {
    use super::*;
    use crate::building_blueprint::tests::{room_sidecar, slab};
    use crate::building_section::{
        CUT_MAX_NY, HeightField, MAX_PLAN_DIM, PLAN_CELL_M, Seg2, VOID_PAD_M, mesh_bounds,
        section_at_owned,
    };
    use crate::bvh::tests::Scene;
    use crate::bvh::{Bvh, BvhSidecar};

    fn farmhouse() -> BvhSidecar {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../packages/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.bvh"
        );
        BvhSidecar::parse(&std::fs::read(path).expect("FarmHouse Wood sidecar"))
            .expect("parse FarmHouse Wood")
    }

    fn tower() -> BvhSidecar {
        // Two stacked boxes so a mid cut misses the upper slab.
        let scenes: [Scene; 2] = [
            slab([0.0, 4.0], [0.0, 3.0], [0.0, 4.0]),
            slab([0.0, 4.0], [6.0, 9.0], [0.0, 4.0]),
        ];
        let (verts, tris) = crate::bvh::tests::concat(&scenes);
        let bvh = Bvh::build(&verts, &tris);
        BvhSidecar::opaque(verts, tris, bvh)
    }

    fn pillar() -> BvhSidecar {
        let sc = slab([1.0, 1.4], [0.0, 2.5], [1.0, 1.4]);
        let (verts, tris) = crate::bvh::tests::concat(&[sc]);
        let bvh = Bvh::build(&verts, &tris);
        BvhSidecar::opaque(verts, tris, bvh)
    }

    /// Six goldens: the committed FarmHouse Wood sidecar plus five Class-R meshes from the
    /// blueprint suite. (`packages/map-assets/everon/prefabs/buildings/` currently materializes
    /// one `.bvh`; the other house shells in `prefabs/blas/` are LFS pointers in this worktree.)
    fn golden_buildings() -> Vec<(&'static str, BvhSidecar)> {
        vec![
            ("FarmHouse_E_1L01_Wood", farmhouse()),
            ("room", room_sidecar(&[])),
            (
                "room_stairwell",
                room_sidecar(&[
                    slab([0.0, 6.0], [2.95, 3.05], [0.0, 1.0]),
                    slab([0.0, 6.0], [2.95, 3.05], [2.0, 6.0]),
                    slab([0.0, 1.0], [2.95, 3.05], [1.0, 2.0]),
                    slab([2.0, 6.0], [2.95, 3.05], [1.0, 2.0]),
                ]),
            ),
            (
                "room_treads",
                room_sidecar(
                    &(0..4)
                        .map(|k| {
                            let x0 = 4.0 + 0.25 * f64::from(k);
                            slab([x0, x0 + 0.25], [0.0, 0.2 + 0.2 * f64::from(k)], [1.0, 2.0])
                        })
                        .collect::<Vec<_>>(),
                ),
            ),
            (
                "room_slope",
                room_sidecar(&[(
                    vec![[0.0, 0.0, 0.0], [6.0, 0.0, 0.0], [0.0, 6.0, 6.0]],
                    vec![[0, 1, 2]],
                )]),
            ),
            ("tower", tower()),
        ]
    }

    fn brute_section_at_owned(
        occl: &BvhSidecar,
        owner: &[u32],
        y: f64,
        max_abs_ny: f64,
    ) -> Vec<(Seg2, u32)> {
        use crate::bvh::{cross, sub};
        let mut out = Vec::new();
        for (ti, &[ia, ib, ic]) in occl.tris.iter().enumerate() {
            let v = [
                occl.verts[ia as usize],
                occl.verts[ib as usize],
                occl.verts[ic as usize],
            ];
            let d = [v[0][1] - y, v[1][1] - y, v[2][1] - y];
            if d.iter().all(|&e| e > 0.0)
                || d.iter().all(|&e| e < 0.0)
                || d.iter().all(|&e| e == 0.0)
            {
                continue;
            }
            let n = cross(sub(v[1], v[0]), sub(v[2], v[0]));
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len < 1e-12 || (n[1] / len).abs() > max_abs_ny {
                continue;
            }
            let mut pts: Vec<[f64; 2]> = Vec::with_capacity(3);
            let mut push = |p: [f64; 2]| {
                if pts.iter().all(|q| (q[0] - p[0]).hypot(q[1] - p[1]) > 1e-9) {
                    pts.push(p);
                }
            };
            for i in 0..3 {
                let j = (i + 1) % 3;
                if d[i] == 0.0 {
                    push([v[i][0], v[i][2]]);
                }
                if d[i] * d[j] < 0.0 {
                    let t = d[i] / (d[i] - d[j]);
                    push([
                        v[i][0] + t * (v[j][0] - v[i][0]),
                        v[i][2] + t * (v[j][2] - v[i][2]),
                    ]);
                }
            }
            if pts.len() >= 2 {
                out.push(([pts[0], pts[1]], owner.get(ti).copied().unwrap_or(0)));
            }
        }
        out
    }

    fn segs_equal(a: &[(Seg2, u32)], b: &[(Seg2, u32)]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        // f32 HeightField store is unrelated; cut geometry stays f64. Exact match after
        // sorting by endpoints so candidate order may differ.
        let mut aa = a.to_vec();
        let mut bb = b.to_vec();
        let key = |s: &(Seg2, u32)| {
            let (p, q) = (s.0[0], s.0[1]);
            let (lo, hi) = if (p[0], p[1]) <= (q[0], q[1]) {
                (p, q)
            } else {
                (q, p)
            };
            (
                s.1,
                lo[0].to_bits(),
                lo[1].to_bits(),
                hi[0].to_bits(),
                hi[1].to_bits(),
            )
        };
        aa.sort_by_key(key);
        bb.sort_by_key(key);
        aa == bb
    }

    #[test]
    fn t938_4_measure_visits_and_bytes() {
        let occl = farmhouse();
        let (lo, hi) = mesh_bounds(&occl).expect("bounds");
        let min = [lo[0] - VOID_PAD_M, lo[1] - VOID_PAD_M];
        let max = [hi[0] + VOID_PAD_M, hi[1] + VOID_PAD_M];
        let empty = HeightField::empty(min, max, PLAN_CELL_M);
        let (_cands, visits) = triangles_overlapping_y_counted(&occl, 1.2, 1.2);
        let built = HeightField::build(&occl, min, max, PLAN_CELL_M, 1.2, 0.2);
        eprintln!(
            "T-938.4 FarmHouse_E_1L01_Wood: tris={} verts={} visits={} empty_bytes={} built_bytes={} cols={} rows={} cap={}",
            occl.tris.len(),
            occl.verts.len(),
            visits,
            empty.allocated_bytes(),
            built.allocated_bytes(),
            empty.cols,
            empty.rows,
            MAX_PLAN_DIM
        );
        assert!(
            visits < occl.tris.len(),
            "index must visit fewer than all {} triangles, got {visits}",
            occl.tris.len()
        );
        assert_eq!(
            empty.allocated_bytes(),
            0,
            "empty HeightField allocates no plan cells"
        );
        let dense = empty.cols * empty.rows * std::mem::size_of::<Option<f64>>();
        assert!(
            built.allocated_bytes() < dense,
            "sparse {} >= dense {}",
            built.allocated_bytes(),
            dense
        );
    }

    #[test]
    fn bvh_root_encloses_section_geometry() {
        for (name, occl) in golden_buildings() {
            assert!(
                bvh_encloses_mesh(&occl),
                "{name}: occlusion BVH root bounds miss mesh verts"
            );
        }
        assert!(bvh_encloses_mesh(&pillar()));
    }

    #[test]
    fn golden_section_cut_equals_brute_force() {
        for (name, occl) in golden_buildings() {
            assert!(bvh_encloses_mesh(&occl), "{name} enclosure");
            for y in [0.45_f64, 1.2, 3.5, 7.0] {
                let got = section_at_owned(&occl, &[], y, CUT_MAX_NY);
                let brute = brute_section_at_owned(&occl, &[], y, CUT_MAX_NY);
                assert!(
                    segs_equal(&got, &brute),
                    "{name} y={y}: indexed {} segs vs brute {}",
                    got.len(),
                    brute.len()
                );
            }
        }
    }

    #[test]
    fn zero_height_inverted_interval_is_empty() {
        let occl = room_sidecar(&[]);
        let (cands, visits) = triangles_overlapping_y_counted(&occl, 1.2, 1.2 - 1e-12);
        assert!(cands.is_empty() && visits == 0);
    }

    #[test]
    fn sparse_heightfield_one_percent_memory() {
        // Cap-sized plan; write a clustered 1% rectangle.
        let span = MAX_PLAN_DIM as f64 * PLAN_CELL_M;
        let mut hf = HeightField::empty([0.0, 0.0], [span, span], PLAN_CELL_M);
        assert_eq!(hf.cols, MAX_PLAN_DIM);
        assert_eq!(hf.rows, MAX_PLAN_DIM);
        assert_eq!(hf.allocated_bytes(), 0);
        let dense = hf.cols * hf.rows * std::mem::size_of::<Option<f64>>();
        let n = (MAX_PLAN_DIM * MAX_PLAN_DIM) / 100;
        let side = (n as f64).sqrt().ceil() as usize;
        for row in 0..side {
            for col in 0..side {
                hf.set(col, row, Some(1.0));
            }
        }
        let sparse = hf.allocated_bytes();
        assert!(
            sparse > 0 && sparse * 50 < dense,
            "1% clustered write should be ~1% of dense: sparse={sparse} dense={dense}"
        );
    }
}
