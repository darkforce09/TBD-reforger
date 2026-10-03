//! The viewshed computed a ray at a time under a time budget.
//!
//! **Role:** [`ViewshedJob`] marches the rays of [`crate::viewshed::compute_viewshed`] in the same
//! order, whole rays between budget checks, and keeps its partial raster readable.
//! **Position:** the Mission Creator's visibility scheduler runs its terrain lane through it; the
//! caller stamps each job with a `generation` and drops a superseded one.
//! **Signals & state:** the job owns its raster and its ray cursor; it holds no clock and no
//! cancellation state beyond `done`: the caller passes the clock and decides whether to step again.
//! **Invariants:** a finished job's raster equals the one-call raster cell for cell; the cursor is
//! always a ray boundary; a cancelled job marches nothing more.

use crate::radial_march::March;
use crate::radial_march::march_schedule;
use crate::viewshed::Viewshed;
use crate::viewshed::ViewshedCapRefused;
use crate::viewshed::ViewshedGrid;
use crate::viewshed::ViewshedParams;
use crate::viewshed::Visibility;
use crate::viewshed::viewshed_grid;
use terrain_elevation::manifest::DemManifest;

/// A viewshed computed in slices: [`ViewshedJob::step`] marches whole rays until its budget is
/// spent and returns, and the caller decides whether to call again. [`ViewshedJob::generation`] is
/// the token a caller stamps and compares (a newer placement drops the older job);
/// [`ViewshedJob::cancel`] retires one in place.
#[derive(Clone, Debug)]
pub struct ViewshedJob {
    /// The elevation model the rays march over.
    pub(crate) manifest: DemManifest,

    /// The raster grid.
    pub(crate) grid: ViewshedGrid,

    /// The observer's world x, metres east.
    pub(crate) obs_x: f64,

    /// The observer's world y, metres north.
    pub(crate) obs_y: f64,

    /// The observer's eye elevation, metres above sea level.
    pub(crate) eye_z: f64,

    /// Rays in the full disc.
    pub(crate) ray_count: usize,

    /// The angle between two rays, radians.
    pub(crate) d_theta: f64,

    /// The step along a ray, metres.
    pub(crate) step_m: f64,

    /// Steps per ray.
    pub(crate) steps: usize,

    /// The raster written so far.
    pub(crate) vs: Viewshed,

    /// The next ray to march: the resume checkpoint, ALWAYS a ray boundary.
    pub cursor: usize,

    /// The caller's cancel token: a newer request carries a newer generation.
    pub generation: u32,

    /// Every ray is marched, or the job was cancelled.
    pub done: bool,
}

impl ViewshedJob {
    /// A job for `p` against `manifest`, stamped with `generation`.
    ///
    /// # Errors
    /// [`ViewshedCapRefused`] when the request's grid holds more than
    /// [`crate::viewshed::MAX_VIEWSHED_CELLS`] cells.
    pub fn new(
        manifest: &DemManifest,
        p: ViewshedParams,
        generation: u32,
    ) -> Result<Self, ViewshedCapRefused> {
        let grid = viewshed_grid(manifest, p);
        grid.cap_check()?;
        let n = grid.cell_count();

        let (cells, eye_z, done) = match p.observer_ground_m {
            None => (vec![Visibility::Unknown; n], 0.0, true),
            Some(g) => (vec![Visibility::Hidden; n], g + p.eye_height_m, false),
        };
        let mut vs = Viewshed {
            cols: grid.cols,
            rows: grid.rows,
            cells,
            min_x: grid.min_x,
            min_y: grid.min_y,
            max_x: grid.max_x,
            max_y: grid.max_y,
            obs_x: p.obs_x,
            obs_y: p.obs_y,
        };
        if p.observer_ground_m.is_some()
            && let Some(oi) = grid.idx_of(p.obs_x, p.obs_y)
        {
            vs.cells[oi] = Visibility::Visible;
        }
        let (ray_count, d_theta, step_m, steps) = march_schedule(grid.cell, grid.radius);
        Ok(Self {
            manifest: *manifest,
            grid,
            obs_x: p.obs_x,
            obs_y: p.obs_y,
            eye_z,
            ray_count,
            d_theta,
            step_m,
            steps,
            vs,
            cursor: 0,
            generation,
            done,
        })
    }
}

impl ViewshedJob {
    /// Marches whole rays, sampling the ground through `elev_at`, until `now()` says
    /// `budget_ms` have passed since the call began; returns whether any ray was marched.
    pub fn step(
        &mut self,
        elev_at: &dyn Fn(f64, f64) -> Option<f64>,
        budget_ms: f64,
        now: &dyn Fn() -> f64,
    ) -> bool {
        if self.done {
            return false;
        }
        let start = now();
        let march = March {
            manifest: &self.manifest,
            grid: self.grid,
            obs_x: self.obs_x,
            obs_y: self.obs_y,
            eye_z: self.eye_z,
        };
        let mut marched = false;
        while self.cursor < self.ray_count {
            let theta = self.cursor as f64 * self.d_theta;
            let (dx, dy) = (theta.cos(), theta.sin());
            let mut max_angle = f64::NEG_INFINITY;
            for s in 1..=self.steps {
                let dist = s as f64 * self.step_m;
                if dist > self.grid.radius {
                    break;
                }
                march.sample(&mut self.vs.cells, (dx, dy), dist, &mut max_angle, elev_at);
            }
            self.cursor += 1;
            marched = true;

            if now() - start >= budget_ms {
                break;
            }
        }
        if self.cursor >= self.ray_count {
            self.done = true;
        }
        marched
    }
}

impl ViewshedJob {
    /// The raster as far as it is marched.
    #[must_use]
    pub fn raster(&self) -> &Viewshed {
        &self.vs
    }
}

impl ViewshedJob {
    /// Take the raster out of a finished (or abandoned) job.
    #[must_use]
    pub fn into_raster(self) -> Viewshed {
        self.vs
    }
}

impl ViewshedJob {
    /// `(rays marched, rays total)` — the progress readout.
    #[must_use]
    pub fn progress(&self) -> (usize, usize) {
        (self.cursor, self.ray_count)
    }
}

impl ViewshedJob {
    /// Retire the job in place: no further ray is marched and [`ViewshedJob::step`] is a no-op. The partial raster stays readable (a caller may still want the observer rect).
    pub fn cancel(&mut self) {
        self.done = true;
    }
}

#[cfg(test)]
#[path = "tests/viewshed_job_tests.rs"]
mod tests;
