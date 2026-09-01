//! T-090.6 — the building viewer's 2D drawing, derived from the COLL trimesh: per-level
//! architectural SECTION CUTS, mesh FLOOR faces, mesh ROOF faces, and the floor-coverage raster
//! that lets a level view show the floor below through its voids.
//!
//! Since step 3 the mesh (the `.bvh` occlusion sidecar) is the structural truth the rays hit;
//! the blueprint JSON's walls/plates are an interpretation of it. A plan drawn from the
//! blueprint therefore cannot show what a ray actually stopped on. This module draws the mesh:
//!
//! * [`section_at`] intersects a horizontal plane `y = const` with every triangle → 2D plan
//!   segments. Cutting at [`CUT_MAIN_M`] above a level's base is the architect's plan: walls
//!   become their true double-line outline, window openings become gaps, mullions, columns and
//!   collision furniture become outlines. A second, lower cut ([`CUT_LOW_M`] — the extractor's
//!   own scan height) draws the wall continuous under the window gaps (sills).
//! * [`faces_between`] selects the near-horizontal triangles in a y-window: a level's slab top
//!   (its floor), or — above the top level — the roof surfaces.
//! * [`CoverageGrid`] rasterises those floor faces so [`through_voids`] can clip a lower level's
//!   cut to where the current level has NO floor: stairwells and double-height voids show the
//!   floor below, solid floor hides it.
//!
//! Winding: the COLL sidecar's index winding is not trustworthy (see `bvh.rs`), so face
//! orientation is judged by `|n.y|` only — both faces of a slab count as "floor", which is
//! harmless for a fill and immune to the inversion trap. Everything here is pure and
//! native-tested; the viewer only packs the results into engine lanes.

use crate::building_blueprint::BuildingBlueprint;
use crate::bvh::{BvhSidecar, cross, sub};

/// Main section cut above a level's base (m): eye height, the architect's plan cut.
pub const CUT_MAIN_M: f64 = 1.2;
/// Low section cut above a level's base (m): the extractor's scan height; sills read here.
pub const CUT_LOW_M: f64 = 0.45;
/// Floor-face window around a level's base (m): slab tops, landings and thresholds; excludes
/// the foundation skirt below and the next slab above.
pub const FLOOR_WINDOW_M: [f64; 2] = [-0.25, 0.6];
/// `|n.y|` floor for a floor face (≈ 60° from horizontal still counts — stair treads, ramps).
pub const FLOOR_MIN_NY: f64 = 0.5;
/// `|n.y|` floor for a roof face (steep pitches and dormer cheeks stay in).
pub const ROOF_MIN_NY: f64 = 0.2;
/// Roof faces live above the top level's base by at least this much (m).
pub const ROOF_ABOVE_TOP_M: f64 = 0.6;
/// Coverage raster pitch (m) — also the ghost-piece length for [`through_voids`].
pub const VOID_CELL_M: f64 = 0.25;
/// Coverage raster padding around the footprint bbox (m).
pub const VOID_PAD_M: f64 = 1.0;

/// A plan segment `[[x, z], [x, z]]` in the building's local frame.
pub type Seg2 = [[f64; 2]; 2];

/// One near-horizontal mesh triangle in plan, with its mean height.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceFill {
    pub tri: [[f64; 2]; 3],
    pub y: f64,
}

/// Boolean floor-occupancy raster over a plan rect: row-major, row 0 = `min_z`, col 0 = `min_x`.
#[derive(Clone, Debug, PartialEq)]
pub struct CoverageGrid {
    pub min_x: f64,
    pub min_z: f64,
    pub cell_m: f64,
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<bool>,
}

impl CoverageGrid {
    /// An all-uncovered grid over `[min, max]` at `cell_m` (at least 1 × 1).
    #[must_use]
    pub fn empty(min: [f64; 2], max: [f64; 2], cell_m: f64) -> Self {
        let cell_m = cell_m.max(1e-3);
        let cols = (((max[0] - min[0]) / cell_m).ceil() as usize).max(1);
        let rows = (((max[1] - min[1]) / cell_m).ceil() as usize).max(1);
        Self {
            min_x: min[0],
            min_z: min[1],
            cell_m,
            cols,
            rows,
            cells: vec![false; cols * rows],
        }
    }

    /// Rasterise `faces` over `[min, max]`: a cell is covered when its centre lies in any face.
    #[must_use]
    pub fn from_faces(min: [f64; 2], max: [f64; 2], cell_m: f64, faces: &[FaceFill]) -> Self {
        let mut g = Self::empty(min, max, cell_m);
        for f in faces {
            let (mut lo, mut hi) = (f.tri[0], f.tri[0]);
            for p in &f.tri[1..] {
                lo = [lo[0].min(p[0]), lo[1].min(p[1])];
                hi = [hi[0].max(p[0]), hi[1].max(p[1])];
            }
            let c0 = (((lo[0] - g.min_x) / g.cell_m).floor().max(0.0)) as usize;
            let r0 = (((lo[1] - g.min_z) / g.cell_m).floor().max(0.0)) as usize;
            let c1 = ((((hi[0] - g.min_x) / g.cell_m).ceil()) as usize).min(g.cols);
            let r1 = ((((hi[1] - g.min_z) / g.cell_m).ceil()) as usize).min(g.rows);
            for row in r0..r1 {
                for col in c0..c1 {
                    let c = g.cell_center(col, row);
                    if point_in_tri(c, f.tri) {
                        g.cells[row * g.cols + col] = true;
                    }
                }
            }
        }
        g
    }

    /// Local `[x, z]` centre of cell `(col, row)`.
    #[must_use]
    pub fn cell_center(&self, col: usize, row: usize) -> [f64; 2] {
        [
            self.min_x + (col as f64 + 0.5) * self.cell_m,
            self.min_z + (row as f64 + 0.5) * self.cell_m,
        ]
    }

    /// Is local `[x, z]` on covered floor? Outside the rect is never covered.
    #[must_use]
    pub fn covered(&self, x: f64, z: f64) -> bool {
        if x < self.min_x || z < self.min_z {
            return false;
        }
        let col = ((x - self.min_x) / self.cell_m) as usize;
        let row = ((z - self.min_z) / self.cell_m) as usize;
        col < self.cols && row < self.rows && self.cells[row * self.cols + col]
    }

    /// Number of covered cells.
    #[must_use]
    pub fn covered_count(&self) -> usize {
        self.cells.iter().filter(|&&c| c).count()
    }
}

/// Inclusive point-in-triangle by signed areas (winding-agnostic).
fn point_in_tri(p: [f64; 2], t: [[f64; 2]; 3]) -> bool {
    let s =
        |a: [f64; 2], b: [f64; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let (d0, d1, d2) = (s(t[0], t[1]), s(t[1], t[2]), s(t[2], t[0]));
    let eps = 1e-12;
    (d0 >= -eps && d1 >= -eps && d2 >= -eps) || (d0 <= eps && d1 <= eps && d2 <= eps)
}

/// One level's mesh drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelDrawing {
    pub level_index: usize,
    /// The level's base (`elevation_range[0]`).
    pub lo: f64,
    pub cut_main_y: f64,
    pub cut_low_y: f64,
    /// Eye-height section: the wall drawing.
    pub cut_main: Vec<Seg2>,
    /// Low section: sills / low walls, dim.
    pub cut_low: Vec<Seg2>,
    /// The slab faces in the floor window — the plate.
    pub floor: Vec<FaceFill>,
    /// Floor occupancy of `floor` — where a lower level's ghost is hidden.
    pub coverage: CoverageGrid,
}

/// The whole building's mesh drawing: one entry per blueprint level (positional) + the roof.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildingDrawing {
    pub levels: Vec<LevelDrawing>,
    /// Near-horizontal faces above the top level: the roof surfaces.
    pub roof: Vec<FaceFill>,
    /// `[min, max]` mean height over `roof` (`[0, 0]` when empty) — the roof ramp's range.
    pub roof_y: [f64; 2],
}

/// Horizontal plane `y` ∩ every triangle → plan segments. A triangle entirely above, below or
/// ON the plane contributes nothing; each crossing edge contributes its interpolated point, a
/// vertex exactly on the plane contributes itself; two distinct points make one segment.
#[must_use]
pub fn section_at(occl: &BvhSidecar, y: f64) -> Vec<Seg2> {
    let mut out = Vec::new();
    for &[ia, ib, ic] in &occl.tris {
        let v = [
            occl.verts[ia as usize],
            occl.verts[ib as usize],
            occl.verts[ic as usize],
        ];
        let d = [v[0][1] - y, v[1][1] - y, v[2][1] - y];
        if d.iter().all(|&e| e > 0.0) || d.iter().all(|&e| e < 0.0) || d.iter().all(|&e| e == 0.0) {
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
            out.push([pts[0], pts[1]]);
        }
    }
    out
}

/// Triangles whose geometric normal has `|n.y| ≥ min_abs_ny` and whose mean height lies in
/// `[y_lo, y_hi]`, as plan faces. Degenerate triangles are skipped.
#[must_use]
pub fn faces_between(occl: &BvhSidecar, y_lo: f64, y_hi: f64, min_abs_ny: f64) -> Vec<FaceFill> {
    let mut out = Vec::new();
    for &[ia, ib, ic] in &occl.tris {
        let (a, b, c) = (
            occl.verts[ia as usize],
            occl.verts[ib as usize],
            occl.verts[ic as usize],
        );
        let n = cross(sub(b, a), sub(c, a));
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len < 1e-12 || (n[1] / len).abs() < min_abs_ny {
            continue;
        }
        let y = (a[1] + b[1] + c[1]) / 3.0;
        if y < y_lo || y > y_hi {
            continue;
        }
        out.push(FaceFill {
            tri: [[a[0], a[2]], [b[0], b[2]], [c[0], c[2]]],
            y,
        });
    }
    out
}

/// Split each segment into pieces of at most `step_m` and keep the pieces whose midpoint is
/// NOT covered — a lower level's cut, visible only through this level's voids.
#[must_use]
pub fn through_voids(segs: &[Seg2], cover: &CoverageGrid, step_m: f64) -> Vec<Seg2> {
    let step = step_m.max(1e-3);
    let mut out = Vec::new();
    for s in segs {
        let len = (s[1][0] - s[0][0]).hypot(s[1][1] - s[0][1]);
        let n = ((len / step).ceil() as usize).max(1);
        let at = |t: f64| {
            [
                s[0][0] + t * (s[1][0] - s[0][0]),
                s[0][1] + t * (s[1][1] - s[0][1]),
            ]
        };
        for k in 0..n {
            let a = at(k as f64 / n as f64);
            let b = at((k + 1) as f64 / n as f64);
            let mid = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
            if !cover.covered(mid[0], mid[1]) {
                out.push([a, b]);
            }
        }
    }
    out
}

/// The mesh drawing for every level of `bp` (positional, carrying `level_index`) plus the
/// roof. Cut heights clamp into short bands (`main ≤ 60 %`, `low ≤ 25 %` of the band).
#[must_use]
pub fn building_drawing(bp: &BuildingBlueprint, occl: &BvhSidecar) -> BuildingDrawing {
    let bb = &bp.overall_footprint.bounding_box2_d;
    let min = [bb.min[0] - VOID_PAD_M, bb.min[1] - VOID_PAD_M];
    let max = [bb.max[0] + VOID_PAD_M, bb.max[1] + VOID_PAD_M];
    let levels = bp
        .levels
        .iter()
        .map(|lvl| {
            let [lo, hi] = lvl.elevation_range;
            let span = (hi - lo).max(0.0);
            let cut_main_y = lo + CUT_MAIN_M.min(0.6 * span);
            let cut_low_y = lo + CUT_LOW_M.min(0.25 * span);
            let floor = faces_between(
                occl,
                lo + FLOOR_WINDOW_M[0],
                lo + FLOOR_WINDOW_M[1],
                FLOOR_MIN_NY,
            );
            let coverage = CoverageGrid::from_faces(min, max, VOID_CELL_M, &floor);
            LevelDrawing {
                level_index: lvl.level_index,
                lo,
                cut_main_y,
                cut_low_y,
                cut_main: section_at(occl, cut_main_y),
                cut_low: section_at(occl, cut_low_y),
                floor,
                coverage,
            }
        })
        .collect();
    let top_lo = bp.levels.last().map_or(0.0, |l| l.elevation_range[0]);
    let roof = faces_between(occl, top_lo + ROOF_ABOVE_TOP_M, f64::INFINITY, ROOF_MIN_NY);
    let roof_y = if roof.is_empty() {
        [0.0, 0.0]
    } else {
        roof.iter()
            .fold([f64::INFINITY, f64::NEG_INFINITY], |acc, f| {
                [acc[0].min(f.y), acc[1].max(f.y)]
            })
    };
    BuildingDrawing {
        levels,
        roof,
        roof_y,
    }
}

#[cfg(test)]
#[path = "building_section_tests.rs"]
mod tests;
