//! **Role:** Shared inputs of the terrain viewshed and viewshed job tests: a uniform elevation
//! manifest and the viewshed parameters built from an observer, a radius and a cell size.
//! **Position:** Test-only sibling of `viewshed_tests.rs` and `scheduler_tests.rs`, mounted from
//! `crate::spatial::los::terrain`; feeds [`crate::spatial::los::terrain::viewshed::compute_viewshed`]
//! and [`crate::spatial::los::terrain::scheduler::ViewshedJob`].
//! **Signals & state:** none; pure functions.
//! **Invariants:** every viewshed fixture puts the eye 1.8 m above the observer's ground.

use crate::spatial::los::terrain::viewshed::ViewshedParams;
use crate::world::terrain::dem::manifest::DemManifest;

/// A one-pixel manifest covering the square `[0, span]` in both axes, heights 0 to 500 m.
pub(super) fn flat_world(span: f64) -> DemManifest {
    DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: span,
        max_y: span,
        width_px: 1,
        height_px: 1,
        flip_x: false,
        flip_z: false,
        height_min_m: 0.0,
        height_max_m: 500.0,
    }
}

/// Viewshed parameters for an observer at `(obs_x, obs_y)` standing on `ground`.
pub(super) fn params(
    obs_x: f64,
    obs_y: f64,
    ground: Option<f64>,
    radius: f64,
    cell: f64,
) -> ViewshedParams {
    ViewshedParams {
        obs_x,
        obs_y,
        observer_ground_m: ground,
        eye_height_m: 1.8,
        radius_m: radius,
        cell_m: cell,
    }
}
