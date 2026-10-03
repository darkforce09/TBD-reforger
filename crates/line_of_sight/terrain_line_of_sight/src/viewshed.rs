//! The viewshed: which cells around an observer the observer's eye sees over the bare ground.
//!
//! **Role:** declares the raster ([`Viewshed`] of [`Visibility`] cells), its request
//! ([`ViewshedParams`]), its grid ([`ViewshedGrid`], [`viewshed_grid`]) and the cell cap
//! ([`MAX_VIEWSHED_CELLS`], [`ViewshedCapRefused`]), and computes the raster in one call
//! ([`compute_viewshed`]).
//! **Position:** marches through the crate's radial march; [`crate::viewshed_job::ViewshedJob`]
//! computes the same raster in slices; the interior line of sight's floor wash reuses
//! [`Visibility`] and [`ViewshedCapRefused`]; the Mission Creator's line-of-sight tool packs the
//! raster into the viewshed texture.
//! **Signals & state:** none; pure functions over an injected elevation sampler.
//! **Invariants:** a raster over [`MAX_VIEWSHED_CELLS`] cells is refused; an observer with no
//! ground yields an all-`Unknown` raster; a cell no ray saw from coverage is never `Visible`.

use crate::radial_march::March;
use crate::radial_march::march_schedule;
use terrain_elevation::manifest::DemManifest;

/// What the observer's eye can tell about one raster cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    /// The observer's eye clears every closer cell on the ray to here — the cell is seen.
    Visible,

    /// A closer ridge rises above the sight line to here — the cell is in dead ground.
    Hidden,

    /// The cell (or the observer) is off DEM coverage, so visibility cannot be judged. A renderer shows it as hidden, never as a fabricated `Visible`.
    Unknown,
}

/// A computed viewshed: a `cols × rows` row-major grid of [`Visibility`] over the world rect `[min_x, min_y]..[max_x, max_y]`, plus the observer world point it was cast from. The raster dimensions match the DEM grid the compute marched (8 m cells in the live editor), so the frontend can turn it straight into an RGBA texture over the same world rect.
#[derive(Clone, Debug, PartialEq)]
pub struct Viewshed {
    /// Raster columns, west to east.
    pub cols: usize,

    /// Raster rows, south to north.
    pub rows: usize,

    /// Row-major, `cols * rows` entries.
    pub cells: Vec<Visibility>,

    /// World-space rect the raster covers (cell centres span the inclusive endpoints).
    pub min_x: f64,

    /// The south edge, world metres north.
    pub min_y: f64,

    /// The east edge, world metres east.
    pub max_x: f64,

    /// The north edge, world metres north.
    pub max_y: f64,

    /// The observer's world x, metres east (the overlay's observer dot and the recompute key).
    pub obs_x: f64,

    /// The observer's world y, metres north.
    pub obs_y: f64,
}

impl Viewshed {
    /// The visibility at cell `(col, row)`, or [`Visibility::Unknown`] out of bounds.
    #[must_use]
    pub fn at(&self, col: usize, row: usize) -> Visibility {
        if col >= self.cols || row >= self.rows {
            return Visibility::Unknown;
        }
        self.cells[row * self.cols + col]
    }
}

impl Viewshed {
    /// Count of cells in each class — `(visible, hidden, unknown)`. Sums to `cols * rows`. The coverage-completeness check for the radial test (every in-radius cell is classified).
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

/// Parameters for [`compute_viewshed`]. Grouped in a struct so a preset (a taller observer eye, a different radius) is a field change, not a churny argument list, and so the radial-step and cell policy are documented in one place.
#[derive(Clone, Copy, Debug)]
pub struct ViewshedParams {
    /// The observer's world x, metres east.
    pub obs_x: f64,

    /// The observer's world y, metres north.
    pub obs_y: f64,

    /// The ground elevation under the observer, metres above sea level; `None` off coverage.
    pub observer_ground_m: Option<f64>,

    /// Eye height above the observer's ground, metres (the LoS `EYE_HEIGHT_OBSERVER_M`).
    pub eye_height_m: f64,

    /// Sight radius, metres (default 2000; adjustable).
    pub radius_m: f64,

    /// Raster cell size, metres — the DEM grid spacing (8 m live). Drives the raster dimensions, the ray step (half a cell) and the angle between rays (half a cell apart at the rim).
    pub cell_m: f64,
}

/// The sight radius a request without a positive radius gets, metres.
pub const VIEWSHED_DEFAULT_RADIUS_M: f64 = 2000.0;

/// The most cells a viewshed raster may hold; the 2000 m / 8 m default holds 251 001.
pub const MAX_VIEWSHED_CELLS: usize = 300_000;

/// A viewshed request a cap refused: which cap, its limit, and the measured value that broke it. One type for both rasters, the terrain cells here and the building wash radius of the interior line of sight, so a caller has one thing to surface; its message names the cap and the number that broke it.
#[derive(Clone, Copy, Debug, PartialEq, thiserror::Error)]
#[error("viewshed refused: {cap} {measured:.0} exceeds the cap of {limit:.0}")]
pub struct ViewshedCapRefused {
    /// What was measured, with its unit — e.g. `"terrain viewshed cells"`.
    pub cap: &'static str,

    /// The cap's limit, in the same unit as `measured`.
    pub limit: f64,

    /// The refused request's measured value.
    pub measured: f64,
}

/// The raster grid of one viewshed request: the resolved cell and radius and the world rectangle,
/// clipped to the elevation model, that the cells cover.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewshedGrid {
    /// Cell pitch, metres (the resolved `cell_m`).
    pub cell: f64,

    /// Sight radius, metres (the resolved `radius_m`).
    pub radius: f64,

    /// The west edge, world metres east.
    pub min_x: f64,

    /// The south edge, world metres north.
    pub min_y: f64,

    /// The east edge, world metres east.
    pub max_x: f64,

    /// The north edge, world metres north.
    pub max_y: f64,

    /// Raster columns, west to east.
    pub cols: usize,

    /// Raster rows, south to north.
    pub rows: usize,
}

/// The grid of request `p` over `manifest`: an 8 m cell and the default radius stand in for a
/// value that is not positive, and the disc's square is clipped to the elevation model.
#[must_use]
pub fn viewshed_grid(manifest: &DemManifest, p: ViewshedParams) -> ViewshedGrid {
    let cell = if p.cell_m.is_finite() && p.cell_m > 0.0 {
        p.cell_m
    } else {
        8.0
    };
    let radius = if p.radius_m.is_finite() && p.radius_m > 0.0 {
        p.radius_m
    } else {
        VIEWSHED_DEFAULT_RADIUS_M
    };
    let min_x = (p.obs_x - radius).max(manifest.min_x);
    let min_y = (p.obs_y - radius).max(manifest.min_y);
    let max_x = (p.obs_x + radius).min(manifest.max_x);
    let max_y = (p.obs_y + radius).min(manifest.max_y);
    let span_x = (max_x - min_x).max(0.0);
    let span_y = (max_y - min_y).max(0.0);
    let cols = ((span_x / cell).round() as usize) + 1;
    let rows = ((span_y / cell).round() as usize) + 1;
    ViewshedGrid {
        cell,
        radius,
        min_x,
        min_y,
        max_x,
        max_y,
        cols,
        rows,
    }
}

impl ViewshedGrid {
    /// Cells in the raster (`cols · rows`, saturating).
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.cols.saturating_mul(self.rows)
    }
}

impl ViewshedGrid {
    /// Refuses a grid over [`MAX_VIEWSHED_CELLS`] cells.
    ///
    /// # Errors
    /// [`ViewshedCapRefused`] naming the terrain cell cap and the grid's cell count.
    pub fn cap_check(&self) -> Result<(), ViewshedCapRefused> {
        let cells = self.cell_count();
        if cells > MAX_VIEWSHED_CELLS {
            return Err(ViewshedCapRefused {
                cap: "terrain viewshed cells",
                limit: MAX_VIEWSHED_CELLS as f64,
                measured: cells as f64,
            });
        }
        Ok(())
    }
}

impl ViewshedGrid {
    /// World `(x, y)` → the nearest raster cell index, or `None` outside the raster.
    #[must_use]
    pub fn idx_of(&self, x: f64, y: f64) -> Option<usize> {
        let c = ((x - self.min_x) / self.cell).round();
        let r = ((y - self.min_y) / self.cell).round();
        if c < 0.0 || r < 0.0 {
            return None;
        }
        let (c, r) = (c as usize, r as usize);
        if c >= self.cols || r >= self.rows {
            return None;
        }
        Some(r * self.cols + c)
    }
}

/// Budget: O(rays × steps) sampler calls — for the 2000 m / 8 m default ≈ 3140 rays × 500 steps (2× oversample), measured well inside ~100 ms with an in-RAM grid sampler.
#[must_use]
pub fn compute_viewshed<F>(manifest: &DemManifest, p: ViewshedParams, elev_at: F) -> Viewshed
where
    F: Fn(f64, f64) -> Option<f64>,
{
    let grid = viewshed_grid(manifest, p);
    let ViewshedGrid {
        min_x,
        min_y,
        max_x,
        max_y,
        cols,
        rows,
        ..
    } = grid;
    let n = grid.cell_count();

    if grid.cap_check().is_err() {
        return Viewshed {
            cols: 0,
            rows: 0,
            cells: Vec::new(),
            min_x,
            min_y,
            max_x,
            max_y,
            obs_x: p.obs_x,
            obs_y: p.obs_y,
        };
    }

    let Some(obs_ground) = p.observer_ground_m else {
        return Viewshed {
            cols,
            rows,
            cells: vec![Visibility::Unknown; n],
            min_x,
            min_y,
            max_x,
            max_y,
            obs_x: p.obs_x,
            obs_y: p.obs_y,
        };
    };
    let eye_z = obs_ground + p.eye_height_m;

    let mut cells = vec![Visibility::Hidden; n];

    if let Some(oi) = grid.idx_of(p.obs_x, p.obs_y) {
        cells[oi] = Visibility::Visible;
    }

    let (ray_count, d_theta, step_m, steps) = march_schedule(grid.cell, grid.radius);
    let march = March {
        manifest,
        grid,
        obs_x: p.obs_x,
        obs_y: p.obs_y,
        eye_z,
    };
    let elev: &dyn Fn(f64, f64) -> Option<f64> = &elev_at;

    for ri in 0..ray_count {
        let theta = ri as f64 * d_theta;
        let (dx, dy) = (theta.cos(), theta.sin());

        let mut max_angle = f64::NEG_INFINITY;
        for s in 1..=steps {
            let dist = s as f64 * step_m;
            if dist > grid.radius {
                break;
            }
            march.sample(&mut cells, (dx, dy), dist, &mut max_angle, elev);
        }
    }

    Viewshed {
        cols,
        rows,
        cells,
        min_x,
        min_y,
        max_x,
        max_y,
        obs_x: p.obs_x,
        obs_y: p.obs_y,
    }
}

#[cfg(test)]
#[path = "tests/viewshed_tests.rs"]
mod tests;
