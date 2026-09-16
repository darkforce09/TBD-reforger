//! Role: viewshed.
//! Position: `spatial/terrain_los` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::terrain_los::march::March;
use crate::spatial::terrain_los::march::march_schedule;
use crate::terrain::dem::manifest::DemManifest;

/// Visibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    /// The observer's eye clears every closer cell on the ray to here — the cell is seen.
    Visible,

    /// A closer ridge rises above the sight line to here — the cell is in dead ground.
    Hidden,

    /// The cell (or the observer) is off DEM coverage, so visibility cannot be judged. Rendered as hidden (a possibly-different alpha; see `los_tool`), NEVER as a fabricated `Visible`.
    Unknown,
}

/// A computed viewshed: a `cols × rows` row-major grid of [`Visibility`] over the world rect `[min_x, min_y]..[max_x, max_y]`, plus the observer world point it was cast from. The raster dimensions match the DEM grid the compute marched (8 m cells in the live editor), so the frontend can turn it straight into an RGBA texture over the same world rect.
#[derive(Clone, Debug, PartialEq)]
pub struct Viewshed {
    /// Cols.
    pub cols: usize,

    /// Rows.
    pub rows: usize,

    /// Row-major, `cols * rows` entries.
    pub cells: Vec<Visibility>,

    /// World-space rect the raster covers (cell centres span the inclusive endpoints).
    pub min_x: f64,

    /// Min y.
    pub min_y: f64,

    /// Max x.
    pub max_x: f64,

    /// Max y.
    pub max_y: f64,

    /// The observer world point (for the overlay's observer dot + re-compute keying).
    pub obs_x: f64,

    /// Obs y.
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
    /// Observer world point.
    pub obs_x: f64,

    /// Obs y.
    pub obs_y: f64,

    /// Observer ground m.
    pub observer_ground_m: Option<f64>,

    /// Eye height above the observer's ground, metres (the LoS `EYE_HEIGHT_OBSERVER_M`).
    pub eye_height_m: f64,

    /// Sight radius, metres (default 2000; adjustable).
    pub radius_m: f64,

    /// Raster cell size, metres — the DEM grid spacing (8 m live). Drives both the raster dims and the ray step (one cell per march step) + angular step (≤1 cell apart at the rim).
    pub cell_m: f64,
}

/// Canonical viewshed default radius m value.
pub const VIEWSHED_DEFAULT_RADIUS_M: f64 = 2000.0;

/// Canonical max viewshed cells value.
pub const MAX_VIEWSHED_CELLS: usize = 300_000;

/// A viewshed request a cap refused: which cap, its limit, and the measured value that broke it. ONE type for both subsystems — terrain cells here, the building-wash radius in [`building_viewshed`](crate::building_viewshed) — so a caller has one thing to surface, and the [`Display`](std::fmt::Display) form always names the cap AND the number that broke it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewshedCapRefused {
    /// What was measured, with its unit — e.g. `"terrain viewshed cells"`.
    pub cap: &'static str,

    /// The cap's limit, in the same unit as `measured`.
    pub limit: f64,

    /// The refused request's measured value.
    pub measured: f64,
}

impl std::fmt::Display for ViewshedCapRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "viewshed refused: {} {:.0} exceeds the cap of {:.0}",
            self.cap, self.measured, self.limit
        )
    }
}

/// Viewshed grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewshedGrid {
    /// Cell pitch, metres (the resolved `cell_m`).
    pub cell: f64,

    /// Sight radius, metres (the resolved `radius_m`).
    pub radius: f64,

    /// Min x.
    pub min_x: f64,

    /// Min y.
    pub min_y: f64,

    /// Max x.
    pub max_x: f64,

    /// Max y.
    pub max_y: f64,

    /// Cols.
    pub cols: usize,

    /// Rows.
    pub rows: usize,
}

/// Viewshed grid.
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
    /// Cap check.
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
