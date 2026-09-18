//! Role: walk the DEM under a shot or an observer: the segment profile and the viewshed raster.
//! Position: `editing/tools/line_of_sight` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: both walks are bounded by the same coverage manifest, so a profile and a disc computed from one observer agree about where data exists.

use crate::spatial::los::terrain::sampler::ProfileSample;
use crate::spatial::los::terrain::viewshed::Viewshed;
use crate::world::terrain::dem::manifest::DemManifest;

use super::capture::LosShot;
use super::host_registry::read_registered_sampler;
use super::terrain_verdict::EYE_HEIGHT_OBSERVER_M;

/// The DEM step (world metres) a profile is walked at. The live 8 m downsampled grid makes 8 m the
/// effective floor; a finer grid would use its own spacing. Kept ≤ the grid cell so no ridge
/// between two samples is missed.
pub const PROFILE_STEP_M: f64 = 8.0;

/// Default sight radius (metres), named at the tool surface so the host chrome and the compute
/// agree on one number.
pub const VIEWSHED_RADIUS_M: f64 =
    crate::spatial::los::terrain::viewshed::VIEWSHED_DEFAULT_RADIUS_M;

/// Compute the viewshed raster for an observer at world `(x, y)` from the registered DEM sampler —
/// the same grid a profile walk reads. `None` when no sampler is registered. The eye is anchored at
/// the OBSERVER's true elevation, read from the sampler at that point; if the observer point is off
/// coverage the raster is all-`Unknown`, never fake-visible. Radius is [`VIEWSHED_RADIUS_M`], cells
/// the grid spacing ([`PROFILE_STEP_M`]).
///
/// This is the ONE call a host makes on observer placement: it wraps [`compute_viewshed`] with the
/// registered sampler and the coverage manifest, so the host holds no DEM types of its own.
#[must_use]
pub fn compute_viewshed_for(obs_x: f64, obs_y: f64) -> Option<Viewshed> {
    let sampler = read_registered_sampler()?;
    let observer_ground_m = sampler(obs_x, obs_y);
    let manifest = everon_manifest();
    let params = crate::spatial::los::terrain::viewshed::ViewshedParams {
        obs_x,
        obs_y,
        observer_ground_m,
        eye_height_m: EYE_HEIGHT_OBSERVER_M,
        radius_m: VIEWSHED_RADIUS_M,
        cell_m: PROFILE_STEP_M,
    };
    Some(crate::spatial::los::terrain::viewshed::compute_viewshed(
        &manifest,
        params,
        move |x, y| sampler(x, y),
    ))
}

/// Build the terrain profile for a shot from the registered DEM sampler. Walks the observer→target
/// segment at [`PROFILE_STEP_M`]. Returns an empty profile when no sampler is registered, which the
/// verdict reads as `Unknown` rather than as a clear sight.
///
/// The manifest is the full coverage box; the sampler itself gates finer coverage (it answers `None`
/// where the grid has no data), so the profile is the covered subset either way.
#[must_use]
pub fn build_profile(shot: &LosShot) -> Vec<ProfileSample> {
    let Some(sampler) = read_registered_sampler() else {
        return Vec::new();
    };
    let manifest = everon_manifest();
    crate::spatial::los::terrain::sampler::sample_segment(
        &manifest,
        (shot.obs_x, shot.obs_y),
        (shot.tgt_x, shot.tgt_y),
        PROFILE_STEP_M,
        move |x, y| sampler(x, y),
    )
}

/// The Everon DEM coverage manifest (world box + raster dims) every walk here bounds itself to.
/// Only the box and dims decide coverage — the injected sampler supplies elevation — so the height
/// range is the published Everon band. Mirrors `assets_v2/terrains/everon/manifest.json`.
///
/// Public because the viewshed scheduler builds the same parameters [`compute_viewshed_for`] does
/// and must bound its job to the SAME manifest.
#[must_use]
pub fn everon_manifest() -> DemManifest {
    DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 12_800.0,
        max_y: 12_800.0,
        width_px: 6400,
        height_px: 6400,
        flip_x: false,
        flip_z: false,
        height_min_m: -204.78,
        height_max_m: 375.53,
    }
}
