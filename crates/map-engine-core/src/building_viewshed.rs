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
//! rather than failing — and the disc RADIUS is capped at [`MAX_WASH_RADIUS_M`] (T-938.5), an
//! over-cap request being refused rather than run: coarsening bounds the RASTER but not the WORK,
//! so without that cap a huge disc still pays 2048² rays per level. [`WashJob`] is the resumable
//! form of the same raster, marched in budgeted batches so a placement never holds a frame.
//! The scanned FarmHouse (radius 30 m) at the defaults is 240 × 240 cells
//! ≈ 45k rays inside the disc per level; the viewer computes one level at a time.

use crate::building_blueprint::BuildingBlueprint;
use crate::building_compound::CompoundBuilding;
use crate::bvh::BvhSidecar;
use crate::dem::sample::{ViewshedCapRefused, Visibility};

/// Default cell pitch (m): the operator's "~0.25 m".
pub const WASH_CELL_M: f64 = 0.25;
/// Default target eye height above each level's floor (m).
pub const WASH_EYE_M: f64 = 1.0;
/// Default disc radius (m) around the observer.
pub const WASH_RADIUS_M: f64 = 25.0;
/// Hard cap on cells per axis; a larger disc coarsens the cell to fit.
pub const MAX_WASH_DIM: usize = 2048;
/// Hard cap on the wash disc RADIUS (m) — T-938.5, operator 2026-09-07. 16× the 25 m default, and
/// far above every shipped caller (the building viewer asks for the footprint diagonal + 5 m).
///
/// This is a separate cap from the terrain viewshed's cell count on purpose: [`MAX_WASH_DIM`]
/// already bounds the RASTER by coarsening the cell, so a cell cap here would refuse nothing — what
/// is unbounded is the WORK, because coarsening keeps 2048 × 2048 = 4.2 M rays per level however
/// large the disc gets. Measured on this crate's `t938_5_defect_probe` before the cap: a 1000 m
/// radius on the two-level room fixture ran 8.4 M rays in **5,387 ms** of blocked thread, and was
/// accepted without complaint.
pub const MAX_WASH_RADIUS_M: f64 = 400.0;

/// `Err` when `radius_m` is over [`MAX_WASH_RADIUS_M`] — the T-938.5 building-wash refusal, naming
/// the cap and the measured radius. The ONE authority: [`WashJob::new`] returns it, and the
/// infallible surfaces ([`wash_band`], [`level_wash`], …) refuse on exactly this predicate.
///
/// A non-finite or non-positive radius is NOT refused here — [`grid_rect`] already floors it to one
/// cell, the long-standing behaviour its callers rely on.
///
/// # Errors
/// [`ViewshedCapRefused`] naming `"building wash radius (m)"` and the measured radius.
pub fn wash_cap_check(radius_m: f64) -> Result<(), ViewshedCapRefused> {
    if radius_m > MAX_WASH_RADIUS_M {
        return Err(ViewshedCapRefused {
            cap: "building wash radius (m)",
            limit: MAX_WASH_RADIUS_M,
            measured: radius_m,
        });
    }
    Ok(())
}

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

    /// This cell's verdict under `blocked` — the ONE per-cell rule, shared by the synchronous
    /// [`wash_band`] and the resumable [`WashJob::step`] (T-938.5) so the two paths can differ only
    /// in HOW they iterate, never in what a cell decides. `Unknown` outside the disc: no ray is cast.
    fn cell_verdict(
        &self,
        col: usize,
        row: usize,
        blocked: &dyn Fn([f64; 3], [f64; 3]) -> bool,
    ) -> Visibility {
        let [x, z] = self.cell_center(col, row);
        if (x - self.obs[0]).hypot(z - self.obs[2]) > self.radius_m {
            return Visibility::Unknown;
        }
        let tgt = [x, self.eye_y, z];
        // A cell centred on the observer is a zero-length segment: clear by definition
        // (nothing can lie between a point and itself).
        if tgt == self.obs || !blocked(self.obs, tgt) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        }
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
    Some(wash_band(
        lvl.level_index,
        lvl.elevation_range[0] + p.eye_m,
        obs,
        p,
        |a, b| {
            occl.bvh
                .any_hit(&occl.verts, &occl.tris, a, b, 0.0, 1.0)
                .is_some()
        },
    ))
}

/// The same wash judged by a [`CompoundBuilding`] — shell + instances, glass and foliage
/// transparent, doors in their current state (T-090.11.4).
#[must_use]
pub fn level_wash_compound(
    bp: &BuildingBlueprint,
    c: &CompoundBuilding,
    obs: [f64; 3],
    level_index: usize,
    p: &WashParams,
) -> Option<LevelWash> {
    let lvl = bp.levels.iter().find(|l| l.level_index == level_index)?;
    Some(compound_wash(
        c,
        obs,
        lvl.elevation_range[0] + p.eye_m,
        lvl.level_index,
        p,
    ))
}

/// One [`LevelWash`] per level of `bp` over the compound (see [`level_wash_compound`]).
#[must_use]
pub fn level_washes_compound(
    bp: &BuildingBlueprint,
    c: &CompoundBuilding,
    obs: [f64; 3],
    p: &WashParams,
) -> Vec<LevelWash> {
    bp.levels
        .iter()
        .filter_map(|l| level_wash_compound(bp, c, obs, l.level_index, p))
        .collect()
}

/// Blueprint-free form of [`level_wash_compound`]: the eye plane is given directly.
#[must_use]
pub fn compound_wash(
    c: &CompoundBuilding,
    obs: [f64; 3],
    eye_y: f64,
    level_index: usize,
    p: &WashParams,
) -> LevelWash {
    wash_band(level_index, eye_y, obs, p, |a, b| c.blocked(a, b))
}

/// The raster's GEOMETRY with no cells: the shared opening for the synchronous [`wash_band`] and
/// the resumable [`WashJob`] (T-938.5), so the two lay their cells on one grid.
fn wash_shell(level_index: usize, eye_y: f64, obs: [f64; 3], p: &WashParams) -> LevelWash {
    let (min_x, min_z, n, cell_m) = grid_rect([obs[0], obs[2]], p.radius_m, p.cell_m);
    let (cols, rows) = (n, n);
    LevelWash {
        level_index,
        eye_y,
        obs,
        radius_m: p.radius_m,
        min_x,
        min_z,
        max_x: min_x + cols as f64 * cell_m,
        max_z: min_z + rows as f64 * cell_m,
        cell_m,
        cols,
        rows,
        cells: Vec::new(),
    }
}

/// The raster itself: every cell's eye point inside the disc, `blocked(obs, eye_point)`
/// deciding `Hidden`.
///
/// **T-938.5 cap.** A radius over [`MAX_WASH_RADIUS_M`] is REFUSED, not run: the returned wash is
/// EMPTY (`cols == rows == 0`, no cells — every `at` / `visibility_at` reads `Unknown`), so no ray
/// is cast and the caller's thread is not held. The refusal MESSAGE (which cap, and the measured
/// radius) is [`wash_cap_check`]'s `Err`, which [`WashJob::new`] returns and the editor's
/// `viewshed_scheduler` surfaces. The cap sits HERE and not in [`grid_rect`]: `grid_rect` coarsens
/// an oversize disc to [`MAX_WASH_DIM`] rather than refusing it, which is its own (tested) contract.
pub fn wash_band(
    level_index: usize,
    eye_y: f64,
    obs: [f64; 3],
    p: &WashParams,
    blocked: impl Fn([f64; 3], [f64; 3]) -> bool,
) -> LevelWash {
    let mut wash = wash_shell(level_index, eye_y, obs, p);
    if wash_cap_check(p.radius_m).is_err() {
        wash.cols = 0;
        wash.rows = 0;
        return wash;
    }
    let (cols, rows) = (wash.cols, wash.rows);
    wash.cells.reserve_exact(cols * rows);
    let blocked: &dyn Fn([f64; 3], [f64; 3]) -> bool = &blocked;
    for row in 0..rows {
        for col in 0..cols {
            let v = wash.cell_verdict(col, row, blocked);
            wash.cells.push(v);
        }
    }
    wash
}

/// Cells a [`WashJob`] marches between budget checks. Reading the clock per cell would cost more
/// than the ray it guards, and a whole ROW is too coarse at the [`MAX_WASH_DIM`] 2048-cell width;
/// 256 cells is ~0.26 ms at the crate's measured ~1.03 µs/ray, comfortably inside a 4 ms budget.
pub const WASH_BATCH_CELLS: usize = 256;

/// T-938.5 — the RESUMABLE, CANCELLABLE form of [`wash_band`]: the same per-cell rule
/// ([`LevelWash::cell_verdict`]) over the same grid, advanced one budgeted batch at a time so the
/// caller's frame is never held for a whole sweep.
///
/// Unlike the terrain march, this loop is **pure over its index** — it carries no running horizon
/// and every cell's verdict is a function of `(col, row)`, `obs`, `eye_y` and `blocked` alone — so a
/// checkpoint is legal at ANY cell and the cells are written by index into a pre-sized raster. All
/// the geometry is fixed by [`wash_shell`] up front, so resuming recomputes nothing.
///
/// The core holds no cancellation state of its own (the `los_world::ObjectPass` split): `step`
/// returns after its budget and the CALLER decides whether to call again. [`WashJob::generation`] is
/// the token a caller stamps and compares; [`WashJob::cancel`] retires one in place.
#[derive(Clone, Debug, PartialEq)]
pub struct WashJob {
    wash: LevelWash,
    total: usize,
    /// The next cell index to decide — the resume checkpoint.
    pub cursor: usize,
    /// The caller's cancel token (the `ObjectPass::generation` idiom).
    pub generation: u32,
    pub done: bool,
}

impl WashJob {
    /// A job over the same grid [`wash_band`] would build, stamped with `generation`.
    ///
    /// # Errors
    /// [`ViewshedCapRefused`] when `p.radius_m` is over [`MAX_WASH_RADIUS_M`] — the same cap
    /// [`wash_band`] enforces, checked BEFORE a cell is allocated, and carrying the message.
    pub fn new(
        level_index: usize,
        eye_y: f64,
        obs: [f64; 3],
        p: &WashParams,
        generation: u32,
    ) -> Result<Self, ViewshedCapRefused> {
        wash_cap_check(p.radius_m)?;
        let mut wash = wash_shell(level_index, eye_y, obs, p);
        let total = wash.cols * wash.rows;
        // Pre-sized and written BY INDEX; every cell is overwritten exactly once, so the sentinel
        // never survives into a finished raster.
        wash.cells = vec![Visibility::Unknown; total];
        Ok(Self {
            wash,
            total,
            cursor: 0,
            generation,
            done: total == 0,
        })
    }

    /// Decide cells in [`WASH_BATCH_CELLS`] batches until `budget_ms` of `now()` time has elapsed or
    /// the disc is finished. Returns whether any cell was decided — ALWAYS at least one batch per
    /// call while the job is live, so a zero budget slices as finely as possible and never spins.
    pub fn step(
        &mut self,
        blocked: &dyn Fn([f64; 3], [f64; 3]) -> bool,
        budget_ms: f64,
        now: &dyn Fn() -> f64,
    ) -> bool {
        if self.done {
            return false;
        }
        let start = now();
        let mut decided = false;
        while self.cursor < self.total {
            let end = (self.cursor + WASH_BATCH_CELLS).min(self.total);
            for i in self.cursor..end {
                let (col, row) = (i % self.wash.cols, i / self.wash.cols);
                let v = self.wash.cell_verdict(col, row, blocked);
                self.wash.cells[i] = v;
            }
            self.cursor = end;
            decided = true;
            if now() - start >= budget_ms {
                break;
            }
        }
        if self.cursor >= self.total {
            self.done = true;
        }
        decided
    }

    /// The raster so far — complete once [`WashJob::done`]; before that the cells from `cursor` on
    /// still carry the `Unknown` sentinel (row-major, so a partial wash is a set of finished ROWS
    /// plus one part-row, which is what a progressive draw should show).
    #[must_use]
    pub fn wash(&self) -> &LevelWash {
        &self.wash
    }

    /// Take the raster out of a finished (or abandoned) job.
    #[must_use]
    pub fn into_wash(self) -> LevelWash {
        self.wash
    }

    /// `(cells decided, cells total)` — the progress readout.
    #[must_use]
    pub fn progress(&self) -> (usize, usize) {
        (self.cursor, self.total)
    }

    /// Retire the job in place: no further cell is decided and [`WashJob::step`] is a no-op.
    pub fn cancel(&mut self) {
        self.done = true;
    }
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
