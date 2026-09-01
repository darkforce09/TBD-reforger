//! T-090.6 step 4 — multi-floor viewshed: per-level visibility rasters over the `.bvh`
//! occlusion sidecar.
//!
//! One observer anywhere in the building's local frame; for a [`BuildingLevel`] a grid of
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
//! The grid is an observer-centred square of half-size `radius_m`; cells farther than the
//! radius from the observer are `Unknown` and get no ray — the wash is a DISC, the way the
//! old boundary fan was, so it reads as "what A sees from here" rather than a building-shaped
//! sheet. `cell_m` square cells, capped at [`MAX_WASH_DIM`] per axis by coarsening the cell
//! rather than failing. The scanned FarmHouse (radius 30 m) at the defaults is 240 × 240 cells
//! ≈ 45k rays inside the disc per level; the viewer computes one level at a time.

use crate::building_blueprint::BuildingBlueprint;
use crate::bvh::BvhSidecar;
use crate::dem::sample::Visibility;

/// Default cell pitch (m): the operator's "~0.25 m".
pub const WASH_CELL_M: f64 = 0.25;
/// Default target eye height above each level's floor (m).
pub const WASH_EYE_M: f64 = 1.0;
/// Default disc radius (m) around the observer.
pub const WASH_RADIUS_M: f64 = 25.0;
/// Hard cap on cells per axis; a larger disc coarsens the cell to fit.
pub const MAX_WASH_DIM: usize = 2048;

/// Sampling parameters for [`level_washes`] / [`level_wash`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WashParams {
    pub cell_m: f64,
    pub eye_m: f64,
    pub radius_m: f64,
}

impl Default for WashParams {
    fn default() -> Self {
        Self {
            cell_m: WASH_CELL_M,
            eye_m: WASH_EYE_M,
            radius_m: WASH_RADIUS_M,
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
    /// Disc radius (m); cells beyond it are `Unknown`.
    pub radius_m: f64,
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

    /// `(visible, hidden, unknown)` cell counts.
    #[must_use]
    pub fn class_counts(&self) -> (usize, usize, usize) {
        let (mut v, mut h, mut u) = (0usize, 0usize, 0usize);
        for c in &self.cells {
            match c {
                Visibility::Visible => v += 1,
                Visibility::Hidden => h += 1,
                Visibility::Unknown => u += 1,
            }
        }
        (v, h, u)
    }
}

/// Grid geometry for an observer-centred disc: `(min_x, min_z, n, cell_m)` for an `n × n`
/// square of side `2 · radius`. The cell coarsens (never the rect shrinks) when the side
/// would exceed [`MAX_WASH_DIM`] cells.
fn grid_rect(obs_xz: [f64; 2], radius_m: f64, cell_m: f64) -> (f64, f64, usize, f64) {
    let r = radius_m.max(cell_m.max(1e-3));
    let mut cell = cell_m.max(1e-3);
    let need = (2.0 * r / cell).ceil();
    if need > MAX_WASH_DIM as f64 {
        cell *= need / MAX_WASH_DIM as f64;
    }
    let n = ((2.0 * r / cell).ceil() as usize).clamp(1, MAX_WASH_DIM);
    (obs_xz[0] - r, obs_xz[1] - r, n, cell)
}

/// The wash of the level whose `level_index` is `level_index` (`None` when `bp` has no such
/// level): the observer's visibility of every cell's eye point inside the disc, judged by the
/// sidecar mesh alone.
#[must_use]
pub fn level_wash(
    bp: &BuildingBlueprint,
    occl: &BvhSidecar,
    obs: [f64; 3],
    level_index: usize,
    p: &WashParams,
) -> Option<LevelWash> {
    let lvl = bp.levels.iter().find(|l| l.level_index == level_index)?;
    let (min_x, min_z, n, cell_m) = grid_rect([obs[0], obs[2]], p.radius_m, p.cell_m);
    let (cols, rows) = (n, n);
    let max_x = min_x + cols as f64 * cell_m;
    let max_z = min_z + rows as f64 * cell_m;
    let eye_y = lvl.elevation_range[0] + p.eye_m;
    let radius_m = p.radius_m;
    let mut wash = LevelWash {
        level_index: lvl.level_index,
        eye_y,
        obs,
        radius_m,
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
            if (x - obs[0]).hypot(z - obs[2]) > radius_m {
                wash.cells.push(Visibility::Unknown);
                continue;
            }
            let tgt = [x, eye_y, z];
            // A cell centred on the observer is a zero-length segment: clear by definition
            // (nothing can lie between a point and itself).
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
    Some(wash)
}

/// One [`LevelWash`] per level of `bp`, in level order (empty for a level-less blueprint).
#[must_use]
pub fn level_washes(
    bp: &BuildingBlueprint,
    occl: &BvhSidecar,
    obs: [f64; 3],
    p: &WashParams,
) -> Vec<LevelWash> {
    bp.levels
        .iter()
        .filter_map(|l| level_wash(bp, occl, obs, l.level_index, p))
        .collect()
}

#[cfg(test)]
#[path = "building_viewshed_tests.rs"]
mod tests;
