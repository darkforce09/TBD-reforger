//! Role: scheduler.
//! Position: `spatial/los/terrain` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::los::terrain::march::March;
use crate::spatial::los::terrain::march::march_schedule;
use crate::spatial::los::terrain::viewshed::Viewshed;
use crate::spatial::los::terrain::viewshed::ViewshedCapRefused;
use crate::spatial::los::terrain::viewshed::ViewshedGrid;
use crate::spatial::los::terrain::viewshed::ViewshedParams;
use crate::spatial::los::terrain::viewshed::Visibility;
use crate::spatial::los::terrain::viewshed::viewshed_grid;
use crate::world::terrain::dem::manifest::DemManifest;

/// The core holds no cancellation state of its own (the `los_world::ObjectPass` split): `step` returns after its budget and the CALLER decides whether to call again. [`ViewshedJob::generation`] is the token a caller stamps and compares — a newer placement bumps it and simply drops the older job (see `viewshed_scheduler`); [`ViewshedJob::cancel`] retires one in place.
#[derive(Clone, Debug)]
pub struct ViewshedJob {
    /// Manifest.
    pub(crate) manifest: DemManifest,

    /// Grid.
    pub(crate) grid: ViewshedGrid,

    /// Obs x.
    pub(crate) obs_x: f64,

    /// Obs y.
    pub(crate) obs_y: f64,

    /// Eye z.
    pub(crate) eye_z: f64,

    /// Ray count.
    pub(crate) ray_count: usize,

    /// D theta.
    pub(crate) d_theta: f64,

    /// Step m.
    pub(crate) step_m: f64,

    /// Steps.
    pub(crate) steps: usize,

    /// Vs.
    pub(crate) vs: Viewshed,

    /// The next ray to march: the resume checkpoint, ALWAYS a ray boundary.
    pub cursor: usize,

    /// The caller's cancel token (the `ObjectPass::generation` idiom).
    pub generation: u32,

    /// Done.
    pub done: bool,
}

impl ViewshedJob {
    /// A job for `p` against `manifest`, stamped with `generation`.
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
    /// Step.
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
    /// Raster.
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
