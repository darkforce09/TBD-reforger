//! The seam between the calculator's input drafts and the map engine's fire-mission solve, run
//! on this device.
//!
//! **Role:** maps the page's drafts (selection, terrain, target, battery, wind, burst height)
//! onto the map engine's [`FireMissionInputs`], hands them to [`solve_fire_mission`] — the one
//! assembler the API re-solves a saved fire mission with — and words each charge row for the
//! page through the map engine's shared
//! [`website_map_engine::data::scenario::ballistics::solution_wording`], the same words the
//! offline browser gate checks the page against.
//! **Position:** between the inputs (`inputs/`) and the page's output; the catalog is the
//! [`BallisticsCatalog`] the catalog source decoded, pinned by its id and version in the inputs.
//! **Signals & state:** none; pure functions over plain values and a borrowed catalog.
//! **Invariants:** nothing here assembles or alters a solution: the [`FireMissionSolution`] is
//! the engine's, byte for byte; every input problem is reported at once, never only the first;
//! the inputs always pin the catalog they are solved against; the laid charge is the chosen one
//! when the operator picked one, else the gun's recommendation; angles are shown in the weapon's
//! mils and in degrees, and the aim azimuth with its deflection and range corrections is what
//! the gun lays.

use super::inputs::battery::{battery_error_message, resolve_battery, GunDraft};
use super::inputs::illumination::{
    burst_height_error_message, parse_burst_height, selected_time_fuze,
};
use super::inputs::positions::{
    position_error_message, resolve_position, MortarTerrain, PositionDraft,
};
use super::inputs::weapon_and_shell::{find_shell, ArmamentSelection, ChargeChoice};
use super::inputs::wind::{parse_wind, wind_error_message, WindDraft};
use crate::v2::core::api::dto::ballistics_catalogs::BallisticsCatalog;
use website_map_engine::data::scenario::ballistics::crest_clearance::TerrainProfile;
use website_map_engine::data::scenario::ballistics::fire_mission::{
    solve_fire_mission, FireMissionInputs, FireMissionSolution,
};
use website_map_engine::data::scenario::ballistics::solution_wording::{
    charge_row_words, ChargeRowWords,
};
pub(crate) use website_map_engine::data::scenario::ballistics::solution_wording::{
    gun_heading, laid_rings, mils_and_degrees,
};
use website_map_engine::data::scenario::ballistics::solver::ChargeSolution;

/// Every input draft of the page, as typed.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MissionDrafts<'drafts> {
    /// Weapon, shell and charge.
    pub(crate) selection: &'drafts ArmamentSelection,
    /// The terrain the positions are on.
    pub(crate) terrain: MortarTerrain,
    /// The target.
    pub(crate) target: &'drafts PositionDraft,
    /// The battery.
    pub(crate) guns: &'drafts [GunDraft],
    /// The wind.
    pub(crate) wind: &'drafts WindDraft,
    /// The burst height text.
    pub(crate) burst_height: &'drafts str,
    /// The terrain under the lead gun's line of fire, when a map has sampled it.
    pub(crate) crest_profile: Option<&'drafts TerrainProfile>,
}

/// A solved fire mission: the inputs it was solved from, the target's grid reference, and the
/// engine's solution.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SolvedMission {
    /// The target's grid reference as typed, trimmed.
    pub(crate) target_grid: String,
    /// What was solved, pinned to the catalog version.
    pub(crate) inputs: FireMissionInputs,
    /// The engine's battery solution.
    pub(crate) solution: FireMissionSolution,
}

/// Maps every draft onto the engine's fire-mission inputs for `catalog`. `height_at(x, y)`
/// samples the terrain.
///
/// # Errors
///
/// Every input problem as a sentence, in the order the inputs are laid out.
pub(crate) fn mission_inputs(
    catalog: &BallisticsCatalog,
    drafts: MissionDrafts<'_>,
    height_at: impl Fn(f64, f64) -> Option<f64>,
) -> Result<FireMissionInputs, Vec<String>> {
    let mut problems = Vec::new();
    let selection = drafts.selection;
    let shell = find_shell(catalog, &selection.shell_id);
    if selection.weapon_id.is_empty() || shell.is_none() {
        problems.push("Pick a weapon and a shell.".to_string());
    }
    let charge_rings = match selection.charge {
        ChargeChoice::Recommended => None,
        ChargeChoice::Rings(rings) => Some(rings),
    };
    if let (Some(rings), Some(shell)) = (charge_rings, shell) {
        if shell.charge(rings).is_none() {
            problems.push(format!("{} has no charge {rings}.", shell.display_name));
        }
    }
    let target = resolve_position(drafts.target, drafts.terrain, &height_at)
        .map_err(|e| problems.push(format!("Target: {}", position_error_message(&e))))
        .ok();
    let guns = resolve_battery(drafts.guns, drafts.terrain, &height_at)
        .map_err(|errors| problems.extend(errors.iter().map(battery_error_message)))
        .ok();
    let wind = parse_wind(drafts.wind)
        .map_err(|e| problems.push(wind_error_message(&e)))
        .ok();
    let fuze = selected_time_fuze(catalog, selection);
    let burst_height_m = parse_burst_height(drafts.burst_height, fuze.as_ref())
        .map_err(|e| problems.push(burst_height_error_message(&e)))
        .ok();
    match (target, guns, wind, burst_height_m) {
        (Some(target), Some(guns), Some(wind), Some(burst_height_m)) if problems.is_empty() => {
            Ok(FireMissionInputs {
                catalog_id: catalog.catalog_id.clone(),
                catalog_version: catalog.catalog_version,
                weapon_id: selection.weapon_id.clone(),
                shell_id: selection.shell_id.clone(),
                charge_rings,
                target,
                guns,
                wind,
                burst_height_m,
                crest_profile: drafts.crest_profile.cloned(),
            })
        }
        _ => Err(problems),
    }
}

/// Maps the drafts and solves them with the engine's [`solve_fire_mission`].
///
/// # Errors
///
/// Every input problem as a sentence, or the engine's refusal of the whole mission.
pub(crate) fn solve_mission(
    catalog: &BallisticsCatalog,
    drafts: MissionDrafts<'_>,
    height_at: impl Fn(f64, f64) -> Option<f64>,
) -> Result<SolvedMission, Vec<String>> {
    let inputs = mission_inputs(catalog, drafts, height_at)?;
    let solution =
        solve_fire_mission(catalog, &inputs).map_err(|refusal| vec![format!("{refusal}.")])?;
    Ok(SolvedMission {
        target_grid: drafts.target.grid.trim().to_string(),
        inputs,
        solution,
    })
}

/// One charge row as the page shows it: the shared [`ChargeRowWords`] and whether the gun lays
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChargeRowText {
    /// "Charge n".
    pub(crate) charge: String,
    /// Whether this is the charge the gun lays.
    pub(crate) laid: bool,
    /// Elevation, or the refusal.
    pub(crate) elevation: String,
    /// Aim azimuth to lay; empty when refused.
    pub(crate) aim_azimuth: String,
    /// Deflection correction; empty when refused.
    pub(crate) deflection: String,
    /// Range correction; empty when refused.
    pub(crate) range_correction: String,
    /// Time of flight; empty when refused.
    pub(crate) time_of_flight: String,
    /// Apex above the gun; empty when refused.
    pub(crate) apex: String,
}

/// The page's row for one charge of a gun laying `laid`, worded by [`charge_row_words`].
pub(crate) fn charge_row_text(row: &ChargeSolution, laid: Option<u32>) -> ChargeRowText {
    let ChargeRowWords {
        charge,
        elevation,
        aim_azimuth,
        deflection,
        range_correction,
        time_of_flight,
        apex,
    } = charge_row_words(row);
    ChargeRowText {
        charge,
        laid: laid == Some(row.rings),
        elevation,
        aim_azimuth,
        deflection,
        range_correction,
        time_of_flight,
        apex,
    }
}

#[cfg(test)]
#[path = "tests/solve_bridge.rs"]
mod tests;
