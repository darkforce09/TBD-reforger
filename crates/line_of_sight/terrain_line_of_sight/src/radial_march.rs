//! The radial march every viewshed casts: the ray and step schedule and the horizon rule for one
//! sample.
//!
//! **Role:** `march_schedule` spaces the rays half a cell apart at the rim and steps half a cell
//! along each (a 2× oversample); `March::sample` classifies one sample of a ray against the
//! steepest slope met so far on it.
//! **Position:** crate-private; the one-call viewshed (`crate::viewshed::compute_viewshed`) and the
//! sliced job (`crate::viewshed_job::ViewshedJob`) march the same rays in the same order through it.
//! **Signals & state:** none; the caller owns the raster and the running horizon of each ray.
//! **Invariants:** a sample off the elevation model's coverage, or one the sampler cannot answer,
//! is never given a height: its cell turns `Unknown` unless a ray already saw it; a cell once
//! `Visible` stays so.

use crate::viewshed::ViewshedGrid;
use crate::viewshed::Visibility;
use terrain_elevation::manifest::DemManifest;
use terrain_elevation::sampling::in_coverage;

/// The march of a raster with `cell`-metre cells out to `radius` metres:
/// `(ray count, angle between rays, step in metres, steps per ray)`.
pub(crate) fn march_schedule(cell: f64, radius: f64) -> (usize, f64, f64, usize) {
    const OVERSAMPLE: f64 = 2.0;
    let ray_count = ((2.0 * std::f64::consts::PI) / (cell / radius) * OVERSAMPLE)
        .ceil()
        .max(1.0) as usize;
    let d_theta = (2.0 * std::f64::consts::PI) / ray_count as f64;
    let step_m = cell / OVERSAMPLE;

    let steps = (radius / step_m).floor().max(1.0) as usize;
    (ray_count, d_theta, step_m, steps)
}

/// One observer's march over the elevation model: its eye and the raster grid it writes.
pub(crate) struct March<'a> {
    /// The elevation model whose coverage bounds every sample.
    pub(crate) manifest: &'a DemManifest,

    /// The raster grid the samples land in.
    pub(crate) grid: ViewshedGrid,

    /// The observer's world x, metres east.
    pub(crate) obs_x: f64,

    /// The observer's world y, metres north.
    pub(crate) obs_y: f64,

    /// The observer's eye elevation, metres above sea level.
    pub(crate) eye_z: f64,
}

impl March<'_> {
    /// Classifies the sample `dist` metres along the ray `(dx, dy)` and raises the ray's running
    /// horizon `max_angle` when the sample's ground stands above it.
    pub(crate) fn sample(
        &self,
        cells: &mut [Visibility],
        (dx, dy): (f64, f64),
        dist: f64,
        max_angle: &mut f64,
        elev_at: &dyn Fn(f64, f64) -> Option<f64>,
    ) {
        let wx = self.obs_x + dx * dist;
        let wy = self.obs_y + dy * dist;
        if !in_coverage(self.manifest, wx, wy) {
            if let Some(i) = self.grid.idx_of(wx, wy)
                && cells[i] != Visibility::Visible
            {
                cells[i] = Visibility::Unknown;
            }
            return;
        }
        let Some(ground) = elev_at(wx, wy) else {
            if let Some(i) = self.grid.idx_of(wx, wy)
                && cells[i] != Visibility::Visible
            {
                cells[i] = Visibility::Unknown;
            }
            return;
        };

        let angle = (ground - self.eye_z) / dist;
        let visible = angle >= *max_angle;
        if let Some(i) = self.grid.idx_of(wx, wy) {
            if visible {
                cells[i] = Visibility::Visible;
            } else if cells[i] != Visibility::Visible {
                cells[i] = Visibility::Hidden;
            }
        }

        if angle > *max_angle {
            *max_angle = angle;
        }
    }
}
