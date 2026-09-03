//! T-090.6 — the building viewer's 2D drawing, derived from the COLL trimesh: a "section box"
//! plan renderer that works for ANY mesh (buildings, but equally rocks, trees, furniture):
//!
//! * [`HeightField`] — a top-down cell raster of the mesh's highest surface at each cell
//!   centre, CLIPPED below a view's cut plane. Painted one quad per cell through a height ramp
//!   it reads as stepped relief: floor mid-tone, treads / sills / lower roofs brighter with
//!   height, stairwell pits and the floors below darker with depth. Any face with
//!   `|n.y| ≥` [`SURFACE_MIN_NY`] counts (floors, treads, roof pitches, table tops); vertical
//!   faces cover no area and are ignored.
//! * [`section_at`] — a horizontal plane `y = const` ∩ the NEAR-VERTICAL triangles
//!   (`|n.y| ≤` [`CUT_MAX_NY`]): walls, mullions, columns, chimney, furniture sides → plan
//!   segments. Cutting at [`CUT_MAIN_M`] above a level's base is the architect's plan (true
//!   double-line walls, window openings as gaps); the low cut ([`CUT_LOW_M`], the extractor's
//!   scan height) draws the wall continuous under the window gaps. Roof pitches and treads
//!   never produce lines — that is what made the first mesh drawing messy.
//! * [`through_voids`] — clip a lower level's cut to the cells where this level's surface is
//!   below its floor window (stairwells, double-height voids), so the floor below shows only
//!   there.
//!
//! Winding: the COLL sidecar's index winding is not trustworthy (see `bvh.rs`), so orientation
//! is judged by `|n.y|` only and barycentric interpolation divides by the SIGNED area — both
//! immune to the inversion trap. Everything here is pure and native-tested; the viewer only
//! packs the results into engine lanes. [`drawing_for`] needs no blueprint at all
//! ([`LevelSpec`]s + a rect); [`building_drawing`] is the blueprint adapter.

use crate::building_blueprint::BuildingBlueprint;
use crate::bvh::{BvhSidecar, cross, sub};

/// Heightfield cell pitch (m) — the stepped-gradient granularity.
pub const PLAN_CELL_M: f64 = 0.2;
/// Main section cut above a level's base (m): eye height, the architect's plan cut.
pub const CUT_MAIN_M: f64 = 1.2;
/// Low section cut above a level's base (m): the extractor's scan height; sills read here.
pub const CUT_LOW_M: f64 = 0.45;
/// `|n.y|` ceiling for a face to be CUT: walls and other near-vertical faces only.
pub const CUT_MAX_NY: f64 = 0.35;
/// `|n.y|` floor for a face to be a SURFACE of the heightfield (steep pitches still count).
pub const SURFACE_MIN_NY: f64 = 0.2;
/// Floor window around a level's base (m): what paints as "floor" (the plate ramp).
pub const FLOOR_WINDOW_M: [f64; 2] = [-0.25, 0.35];
/// How far below the floor window the "pit" ramp reaches (m).
pub const PIT_DEPTH_M: f64 = 3.0;
/// Raster padding around the drawing rect (m).
pub const VOID_PAD_M: f64 = 1.0;
/// Hard cap on heightfield cells per axis; a larger rect coarsens the cell to fit.
pub const MAX_PLAN_DIM: usize = 2048;

/// A plan segment `[[x, z], [x, z]]` in the building's local frame.
pub type Seg2 = [[f64; 2]; 2];

/// Top-down raster of the mesh's highest surface per cell (`None` = no surface), row-major,
/// row 0 = `min_z`, col 0 = `min_x`.
#[derive(Clone, Debug, PartialEq)]
pub struct HeightField {
    pub min_x: f64,
    pub min_z: f64,
    pub cell_m: f64,
    pub cols: usize,
    pub rows: usize,
    pub h: Vec<Option<f64>>,
}

impl HeightField {
    /// An all-`None` field over `[min, max]` at `cell_m` (coarsened to fit [`MAX_PLAN_DIM`]).
    #[must_use]
    pub fn empty(min: [f64; 2], max: [f64; 2], cell_m: f64) -> Self {
        let mut cell = cell_m.max(1e-3);
        let span_x = (max[0] - min[0]).max(cell);
        let span_z = (max[1] - min[1]).max(cell);
        let need = (span_x / cell).ceil().max((span_z / cell).ceil());
        if need > MAX_PLAN_DIM as f64 {
            cell *= need / MAX_PLAN_DIM as f64;
        }
        let cols = ((span_x / cell).ceil() as usize).clamp(1, MAX_PLAN_DIM);
        let rows = ((span_z / cell).ceil() as usize).clamp(1, MAX_PLAN_DIM);
        Self {
            min_x: min[0],
            min_z: min[1],
            cell_m: cell,
            cols,
            rows,
            h: vec![None; cols * rows],
        }
    }

    /// Rasterise the mesh's highest surface per cell, keeping only surfaces at or below
    /// `clip_below_y` (a view's cut plane; `f64::INFINITY` for the full top surface). A face
    /// contributes where its `|n.y| ≥ min_abs_ny`; the height at a cell centre is the face
    /// plane's height there (barycentric over the plan triangle).
    #[must_use]
    pub fn build(
        occl: &BvhSidecar,
        min: [f64; 2],
        max: [f64; 2],
        cell_m: f64,
        clip_below_y: f64,
        min_abs_ny: f64,
    ) -> Self {
        let mut hf = Self::empty(min, max, cell_m);
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
            if a[1].min(b[1]).min(c[1]) > clip_below_y {
                continue;
            }
            let (p0, p1, p2) = ([a[0], a[2]], [b[0], b[2]], [c[0], c[2]]);
            let area2 = edge(p0, p1, p2);
            if area2.abs() < 1e-12 {
                continue;
            }
            let lo = [p0[0].min(p1[0]).min(p2[0]), p0[1].min(p1[1]).min(p2[1])];
            let hi = [p0[0].max(p1[0]).max(p2[0]), p0[1].max(p1[1]).max(p2[1])];
            let c0 = (((lo[0] - hf.min_x) / hf.cell_m).floor().max(0.0)) as usize;
            let r0 = (((lo[1] - hf.min_z) / hf.cell_m).floor().max(0.0)) as usize;
            let c1 = ((((hi[0] - hf.min_x) / hf.cell_m).ceil()).max(0.0) as usize).min(hf.cols);
            let r1 = ((((hi[1] - hf.min_z) / hf.cell_m).ceil()).max(0.0) as usize).min(hf.rows);
            for row in r0..r1 {
                for col in c0..c1 {
                    let p = hf.cell_center(col, row);
                    let w0 = edge(p1, p2, p) / area2;
                    let w1 = edge(p2, p0, p) / area2;
                    let w2 = edge(p0, p1, p) / area2;
                    let eps = -1e-9;
                    if w0 < eps || w1 < eps || w2 < eps {
                        continue;
                    }
                    let y = w0 * a[1] + w1 * b[1] + w2 * c[1];
                    if y > clip_below_y {
                        continue;
                    }
                    let slot = &mut hf.h[row * hf.cols + col];
                    *slot = Some(slot.map_or(y, |cur: f64| cur.max(y)));
                }
            }
        }
        hf
    }

    /// Surface height at `(col, row)`; `None` out of bounds or no surface.
    #[must_use]
    pub fn at(&self, col: usize, row: usize) -> Option<f64> {
        if col >= self.cols || row >= self.rows {
            return None;
        }
        self.h[row * self.cols + col]
    }

    /// Local `[x, z]` centre of cell `(col, row)`.
    #[must_use]
    pub fn cell_center(&self, col: usize, row: usize) -> [f64; 2] {
        [
            self.min_x + (col as f64 + 0.5) * self.cell_m,
            self.min_z + (row as f64 + 0.5) * self.cell_m,
        ]
    }

    /// The cell containing local `[x, z]`, or `None` outside the rect.
    #[must_use]
    pub fn cell_at(&self, x: f64, z: f64) -> Option<(usize, usize)> {
        if x < self.min_x || z < self.min_z {
            return None;
        }
        let col = ((x - self.min_x) / self.cell_m) as usize;
        let row = ((z - self.min_z) / self.cell_m) as usize;
        (col < self.cols && row < self.rows).then_some((col, row))
    }

    /// Surface height of the cell containing local `[x, z]`.
    #[must_use]
    pub fn value_at(&self, x: f64, z: f64) -> Option<f64> {
        self.cell_at(x, z).and_then(|(c, r)| self.at(c, r))
    }

    /// Is there a surface at or above `min_y` under local `[x, z]`? Outside the rect: no.
    #[must_use]
    pub fn covered(&self, x: f64, z: f64, min_y: f64) -> bool {
        self.value_at(x, z).is_some_and(|y| y >= min_y)
    }

    /// `[min, max]` over the field's surfaces; `None` when the field is empty.
    #[must_use]
    pub fn range(&self) -> Option<[f64; 2]> {
        self.h
            .iter()
            .flatten()
            .fold(None, |acc: Option<[f64; 2]>, &y| {
                Some(acc.map_or([y, y], |r| [r[0].min(y), r[1].max(y)]))
            })
    }

    /// Number of cells with a surface at or above `min_y`.
    #[must_use]
    pub fn covered_count(&self, min_y: f64) -> usize {
        self.h
            .iter()
            .filter(|c| c.is_some_and(|y| y >= min_y))
            .count()
    }
}

/// Twice the signed area of `(a, b, p)` in the plan.
fn edge(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}

/// A level band for [`drawing_for`] — what a blueprint level reduces to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelSpec {
    pub index: usize,
    pub lo: f64,
    pub hi: f64,
}

/// One level's mesh drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelDrawing {
    pub level_index: usize,
    pub lo: f64,
    pub hi: f64,
    pub cut_main_y: f64,
    pub cut_low_y: f64,
    /// Eye-height section of the vertical faces: the wall drawing.
    pub cut_main: Vec<Seg2>,
    /// Low section: sills / low walls, dim.
    pub cut_low: Vec<Seg2>,
    /// The highest surface per cell below the main cut plane.
    pub surface: HeightField,
}

impl LevelDrawing {
    /// The bottom of this level's floor window — below it a cell is a void / pit.
    #[must_use]
    pub fn floor_min_y(&self) -> f64 {
        self.lo + FLOOR_WINDOW_M[0]
    }
}

/// The whole building's mesh drawing: one entry per level spec (in order) + the roof.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildingDrawing {
    pub levels: Vec<LevelDrawing>,
    /// The full, unclipped top surface: the roof plan.
    pub roof: HeightField,
    /// `[min, max]` over `roof` (`[0, 0]` when empty) — the roof ramp's fallback range.
    pub roof_y: [f64; 2],
}

/// XZ bounds of the mesh (`None` for an empty vertex table).
#[must_use]
pub fn mesh_bounds(occl: &BvhSidecar) -> Option<([f64; 2], [f64; 2])> {
    let mut it = occl.verts.iter();
    let first = it.next()?;
    let (mut lo, mut hi) = ([first[0], first[2]], [first[0], first[2]]);
    for v in it {
        lo = [lo[0].min(v[0]), lo[1].min(v[2])];
        hi = [hi[0].max(v[0]), hi[1].max(v[2])];
    }
    Some((lo, hi))
}

/// Horizontal plane `y` ∩ every triangle with `|n.y| ≤ max_abs_ny` → plan segments. A
/// triangle entirely above, below or ON the plane contributes nothing; each crossing edge
/// contributes its interpolated point, a vertex exactly on the plane contributes itself; two
/// distinct points make one segment.
#[must_use]
pub fn section_at(occl: &BvhSidecar, y: f64, max_abs_ny: f64) -> Vec<Seg2> {
    section_at_owned(occl, &[], y, max_abs_ny)
        .into_iter()
        .map(|(s, _)| s)
        .collect()
}

/// [`section_at`] with the owner of every cut segment: `owner[tri]` (a
/// [`crate::building_compound::FlatMesh`]'s table — `0` shell, `i + 1` instance `i`; missing
/// entries read as `0`), so the viewer routes wall, leaf, frame, pane and furniture cuts to
/// their own lanes (T-090.11.4).
#[must_use]
pub fn section_at_owned(
    occl: &BvhSidecar,
    owner: &[u32],
    y: f64,
    max_abs_ny: f64,
) -> Vec<(Seg2, u32)> {
    let mut out = Vec::new();
    for (ti, &[ia, ib, ic]) in occl.tris.iter().enumerate() {
        let v = [
            occl.verts[ia as usize],
            occl.verts[ib as usize],
            occl.verts[ic as usize],
        ];
        let d = [v[0][1] - y, v[1][1] - y, v[2][1] - y];
        if d.iter().all(|&e| e > 0.0) || d.iter().all(|&e| e < 0.0) || d.iter().all(|&e| e == 0.0) {
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

/// Split each segment into pieces of at most `step_m` and keep the pieces whose midpoint has
/// NO surface at or above `floor_min_y` in `surface` — a lower level's cut, visible only
/// through this level's voids.
#[must_use]
pub fn through_voids(
    segs: &[Seg2],
    surface: &HeightField,
    floor_min_y: f64,
    step_m: f64,
) -> Vec<Seg2> {
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
            if !surface.covered(mid[0], mid[1], floor_min_y) {
                out.push([a, b]);
            }
        }
    }
    out
}

/// The mesh drawing for a set of level bands over the plan rect `[min, max]` (already padded).
/// Cut heights clamp into short bands (`main ≤ 60 %`, `low ≤ 25 %` of the band). Needs no
/// blueprint: any mesh + any bands.
#[must_use]
pub fn drawing_for(
    occl: &BvhSidecar,
    specs: &[LevelSpec],
    min: [f64; 2],
    max: [f64; 2],
) -> BuildingDrawing {
    let levels = specs
        .iter()
        .map(|s| {
            let span = (s.hi - s.lo).max(0.0);
            let cut_main_y = s.lo + CUT_MAIN_M.min(0.6 * span);
            let cut_low_y = s.lo + CUT_LOW_M.min(0.25 * span);
            LevelDrawing {
                level_index: s.index,
                lo: s.lo,
                hi: s.hi,
                cut_main_y,
                cut_low_y,
                cut_main: section_at(occl, cut_main_y, CUT_MAX_NY),
                cut_low: section_at(occl, cut_low_y, CUT_MAX_NY),
                surface: HeightField::build(
                    occl,
                    min,
                    max,
                    PLAN_CELL_M,
                    cut_main_y,
                    SURFACE_MIN_NY,
                ),
            }
        })
        .collect();
    let roof = HeightField::build(occl, min, max, PLAN_CELL_M, f64::INFINITY, SURFACE_MIN_NY);
    let roof_y = roof.range().unwrap_or([0.0, 0.0]);
    BuildingDrawing {
        levels,
        roof,
        roof_y,
    }
}

/// Blueprint adapter for [`drawing_for`]: one spec per blueprint level (positional, carrying
/// `level_index`) over the union of the footprint bbox and the mesh bounds, padded.
#[must_use]
pub fn building_drawing(bp: &BuildingBlueprint, occl: &BvhSidecar) -> BuildingDrawing {
    let bb = &bp.overall_footprint.bounding_box2_d;
    let (mut min, mut max) = (bb.min, bb.max);
    if let Some((lo, hi)) = mesh_bounds(occl) {
        min = [min[0].min(lo[0]), min[1].min(lo[1])];
        max = [max[0].max(hi[0]), max[1].max(hi[1])];
    }
    let min = [min[0] - VOID_PAD_M, min[1] - VOID_PAD_M];
    let max = [max[0] + VOID_PAD_M, max[1] + VOID_PAD_M];
    let specs: Vec<LevelSpec> = bp
        .levels
        .iter()
        .map(|l| LevelSpec {
            index: l.level_index,
            lo: l.elevation_range[0],
            hi: l.elevation_range[1],
        })
        .collect();
    drawing_for(occl, &specs, min, max)
}

#[cfg(test)]
#[path = "building_section_tests.rs"]
mod tests;
