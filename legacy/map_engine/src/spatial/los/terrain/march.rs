//! Role: march.
//! Position: `spatial/los/terrain` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::los::terrain::viewshed::ViewshedGrid;
use crate::spatial::los::terrain::viewshed::Visibility;
use crate::world::terrain::dem::manifest::DemManifest;
use crate::world::terrain::dem::sampling::in_coverage;

/// March schedule.
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

/// March.
pub(crate) struct March<'a> {
    /// Manifest.
    pub(crate) manifest: &'a DemManifest,

    /// Grid.
    pub(crate) grid: ViewshedGrid,

    /// Obs x.
    pub(crate) obs_x: f64,

    /// Obs y.
    pub(crate) obs_y: f64,

    /// Eye z.
    pub(crate) eye_z: f64,
}

impl March<'_> {
    /// Sample.
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
