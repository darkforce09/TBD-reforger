//! The terrain under the lead gun's line of fire, for the crest-clearance check.
//!
//! **Role:** samples the ground from the lead gun to the target into the map engine's
//! [`TerrainProfile`] through
//! [`terrain_line_of_sight::elevation_profile::sample_segment`].
//! **Position:** read by the page on Calculate; the profile goes into the solve bridge's drafts,
//! and the engine's fire-mission assembler measures the lead gun's clearance over it.
//! **Signals & state:** none; the height reader is passed in.
//! **Invariants:** only a terrain with a served elevation model is sampled; a sample the height
//! reader cannot answer is left out, never filled with a guessed height; a profile with no
//! sample is no profile, so an unloaded map means no crest check rather than a refused solve;
//! downrange is measured from the lead gun, in metres, ascending.

#[cfg(any(target_arch = "wasm32", test))]
use crate::pages::field_tools::mortar::inputs::battery::GunDraft;
#[cfg(any(target_arch = "wasm32", test))]
use crate::pages::field_tools::mortar::inputs::positions::MortarTerrain;
#[cfg(any(target_arch = "wasm32", test))]
use crate::pages::field_tools::mortar::inputs::positions::PositionDraft;
#[cfg(any(target_arch = "wasm32", test))]
use map_coordinates::grid_reference::parse_grid;
#[cfg(any(target_arch = "wasm32", test))]
use map_engine::data::scenario::ballistics::crest_clearance::{TerrainProfile, TerrainSample};
#[cfg(any(target_arch = "wasm32", test))]
use map_engine::editing::tools::line_of_sight::terrain_survey::everon_manifest;
#[cfg(any(target_arch = "wasm32", test))]
use terrain_elevation::manifest::DemManifest;
#[cfg(any(target_arch = "wasm32", test))]
use terrain_line_of_sight::elevation_profile::sample_segment;

/// Spacing of the profile samples, metres: the elevation model's native resolution, so no crest
/// narrower than a raster cell falls between two samples.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) const PROFILE_STEP_M: f64 = 2.0;

/// The elevation model coverage of `terrain`; `None` for a terrain without one.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn coverage_manifest(terrain: MortarTerrain) -> Option<DemManifest> {
    match terrain {
        MortarTerrain::Everon => Some(everon_manifest()),
        MortarTerrain::Arland => None,
    }
}

/// The terrain from the lead gun (`guns[0]`) to the target on `terrain`; `height_at(x, y)` is the
/// ground height. `None` when the terrain has no elevation model, a grid reference does not parse,
/// or no sample has a height.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn lead_gun_profile(
    terrain: MortarTerrain,
    target: &PositionDraft,
    guns: &[GunDraft],
    height_at: impl Fn(f64, f64) -> Option<f64>,
) -> Option<TerrainProfile> {
    let manifest = coverage_manifest(terrain)?;
    let lead = parse_grid(&guns.first()?.position.grid).ok()?;
    let aim = parse_grid(&target.grid).ok()?;
    let samples: Vec<TerrainSample> =
        sample_segment(&manifest, lead, aim, PROFILE_STEP_M, |x, y| {
            height_at(x, y).filter(|h| h.is_finite())
        })
        .into_iter()
        .map(|sample| TerrainSample {
            downrange_m: sample.dist_m,
            height_m: sample.elev_m,
        })
        .collect();
    (!samples.is_empty()).then_some(TerrainProfile { samples })
}
