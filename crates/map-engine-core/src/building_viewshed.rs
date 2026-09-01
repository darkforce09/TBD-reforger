//! T-090.6 step 4 — multi-floor viewshed: per-level visibility rasters over the `.bvh`
//! occlusion sidecar.
//!
//! One observer anywhere in the building's local frame; for EVERY [`BuildingLevel`] a grid of
//! target points at eye height above that level's floor (`elevation_range[0] + eye_m`), one
//! [`Bvh::any_hit`](crate::bvh::Bvh::any_hit) ray per cell, `Visible` when nothing in the COLL
//! mesh lies between. The mesh is the whole truth: stairwells, floor and ceiling slabs, rotated
//! furniture that is part of the collision mesh, window holes and their frames all fall out of
//! the raycast — no per-level band logic, no aperture semantics. (Compare
//! [`BuildingBlueprint::evaluate_los`], which pays for attribution + annotations a wash does not
//! need; `any_hit` is an order of magnitude cheaper per ray and decides existence by the same
//! range test.)
//!
//! # Raster contract
//!
//! `cells` is row-major with **row 0 = NORTH (max z)** and **col 0 = min x** — the engine's
//! texture-lane convention (texture row 0 is the world MAX-y edge, and the viewer maps local
//! +z onto world +y), so a wash uploads without a row flip. [`LevelWash::cell_center`] is the
//! one authority for the cell ↔ point mapping; consumers and tests go through it (and its
//! inverse, [`LevelWash::cell_at`]).
//!
//! # Envelope
//!
//! The grid is the overall footprint bbox padded by `pad_m` on every side (what a window shows
//! outside is part of the answer), `cell_m` square cells, capped at [`MAX_WASH_DIM`] per axis by
//! coarsening the cell rather than failing. The scanned FarmHouse (15 × 20 m) at the defaults is
//! 100 × 120 cells × 3 levels = 36k rays per recompute.

use crate::building_blueprint::{BBox2D, BuildingBlueprint};
use crate::bvh::BvhSidecar;
use crate::dem::sample::Visibility;

/// Default cell pitch (m): the operator's "~0.25 m".
pub const WASH_CELL_M: f64 = 0.25;
/// Default target eye height above each level's floor (m).
pub const WASH_EYE_M: f64 = 1.0;
/// Default padding around the footprint bbox (m) — the exterior cells a window reveals.
pub const WASH_PAD_M: f64 = 5.0;
/// Hard cap on cells per axis; a larger footprint coarsens the cell to fit.
pub const MAX_WASH_DIM: usize = 2048;

/// Sampling parameters for [`level_washes`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WashParams {
    pub cell_m: f64,
    pub eye_m: f64,
    pub pad_m: f64,
}

impl Default for WashParams {
    fn default() -> Self {
        Self {
            cell_m: WASH_CELL_M,
            eye_m: WASH_EYE_M,
            pad_m: WASH_PAD_M,
        }
    }
}

/// One level's visibility raster at eye height above its floor.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelWash {
    pub level_index: usize,
    /// Target plane: `elevation_range[0] + eye_m`.
    pub eye_y: f64,
    /// The observer the raster was cast from (recompute keying, overlay dot).
    pub obs: [f64; 3],
    /// Local plan rect the raster covers (cell edges; `max = min + n · cell_m`).
    pub min_x: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_z: f64,
    pub cell_m: f64,
    pub cols: usize,
    pub rows: usize,
    /// Row-major `rows × cols`; row 0 = north (max z), col 0 = min x.
    pub cells: Vec<Visibility>,
}

impl LevelWash {
    /// Visibility at `(col, row)`; `Unknown` out of bounds.
    #[must_use]
    pub fn at(&self, col: usize, row: usize) -> Visibility {
        if col >= self.cols || row >= self.rows {
            return Visibility::Unknown;
        }
        self.cells[row * self.cols + col]
    }

    /// Local `[x, z]` centre of cell `(col, row)` — the point its ray was cast to.
    #[must_use]
    pub fn cell_center(&self, col: usize, row: usize) -> [f64; 2] {
        [
            self.min_x + (col as f64 + 0.5) * self.cell_m,
            self.max_z - (row as f64 + 0.5) * self.cell_m,
        ]
    }

    /// The cell containing local `[x, z]`, or `None` outside the rect (the north and west
    /// edges belong to row 0 / col 0; the south and east edges are exclusive).
    #[must_use]
    pub fn cell_at(&self, x: f64, z: f64) -> Option<(usize, usize)> {
        if x < self.min_x || x >= self.max_x || z <= self.min_z || z > self.max_z {
            return None;
        }
        let col = ((x - self.min_x) / self.cell_m) as usize;
        let row = ((self.max_z - z) / self.cell_m) as usize;
        (col < self.cols && row < self.rows).then_some((col, row))
    }

    /// Visibility of the cell containing local `[x, z]` (`Unknown` outside the rect).
    #[must_use]
    pub fn visibility_at(&self, x: f64, z: f64) -> Visibility {
        self.cell_at(x, z)
            .map_or(Visibility::Unknown, |(c, r)| self.at(c, r))
    }

    /// `(visible, hidden)` cell counts.
    #[must_use]
    pub fn class_counts(&self) -> (usize, usize) {
        let visible = self
            .cells
            .iter()
            .filter(|&&c| c == Visibility::Visible)
            .count();
        (visible, self.cells.len() - visible)
    }
}

/// Grid geometry for a padded bbox: `(min_x, min_z, cols, rows, cell_m)`. The cell coarsens
/// (never the rect shrinks) when either axis would exceed [`MAX_WASH_DIM`].
fn grid_rect(bb: &BBox2D, cell_m: f64, pad_m: f64) -> (f64, f64, usize, usize, f64) {
    let min_x = bb.min[0] - pad_m;
    let min_z = bb.min[1] - pad_m;
    let mut cell = cell_m.max(1e-3);
    let span_x = (bb.max[0] - bb.min[0] + 2.0 * pad_m).max(cell);
    let span_z = (bb.max[1] - bb.min[1] + 2.0 * pad_m).max(cell);
    let need = (span_x / cell).ceil().max((span_z / cell).ceil());
    if need > MAX_WASH_DIM as f64 {
        cell *= need / MAX_WASH_DIM as f64;
    }
    let cols = ((span_x / cell).ceil() as usize).clamp(1, MAX_WASH_DIM);
    let rows = ((span_z / cell).ceil() as usize).clamp(1, MAX_WASH_DIM);
    (min_x, min_z, cols, rows, cell)
}

/// One [`LevelWash`] per level of `bp`, in level order (empty for a level-less blueprint): the
/// observer's visibility of every cell's eye point, judged by the sidecar mesh alone.
#[must_use]
pub fn level_washes(
    bp: &BuildingBlueprint,
    occl: &BvhSidecar,
    obs: [f64; 3],
    p: &WashParams,
) -> Vec<LevelWash> {
    let (min_x, min_z, cols, rows, cell_m) =
        grid_rect(&bp.overall_footprint.bounding_box2_d, p.cell_m, p.pad_m);
    let max_x = min_x + cols as f64 * cell_m;
    let max_z = min_z + rows as f64 * cell_m;
    bp.levels
        .iter()
        .map(|lvl| {
            let eye_y = lvl.elevation_range[0] + p.eye_m;
            let mut wash = LevelWash {
                level_index: lvl.level_index,
                eye_y,
                obs,
                min_x,
                min_z,
                max_x,
                max_z,
                cell_m,
                cols,
                rows,
                cells: Vec::with_capacity(cols * rows),
            };
            for row in 0..rows {
                for col in 0..cols {
                    let [x, z] = wash.cell_center(col, row);
                    let tgt = [x, eye_y, z];
                    // A cell centred on the observer is a zero-length segment: clear by
                    // definition (nothing can lie between a point and itself).
                    let clear = tgt == obs
                        || occl
                            .bvh
                            .any_hit(&occl.verts, &occl.tris, obs, tgt, 0.0, 1.0)
                            .is_none();
                    wash.cells.push(if clear {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    });
                }
            }
            wash
        })
        .collect()
}

#[cfg(test)]
#[path = "building_viewshed_tests.rs"]
mod tests;
