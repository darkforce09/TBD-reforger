//! The native solve of the gate's fire mission and the words the page shows for it.
//!
//! **Role:** solves the mission the gate typed with `fire_mission_planning`'s `solve_fire_mission` (the
//! one assembler the page and the API use), words it with its `solution_wording`
//! (the functions the mortar calculator's battery table and charge tables render), and compares
//! those tables with the ones read from the page.
//! **Position:** after `super::page_driver` has entered the mission and read the tables; the
//! inputs come from `super::mission_plan` and the grids the page holds.
//! **Signals & state:** none; pure functions over plain values.
//! **Invariants:** the native figures are worded by the same functions as the page's, so a
//! table matches only when the page's solve equals the native one at every shown digit; a
//! table matches only cell for cell, row for row and with the same laid charge; the crest line
//! is not compared, because the page samples it from the elevation model and the gate types
//! heights.

use crate::Result;
use crate::error::refusal;
use ballistics_model::catalog::BallisticsCatalog;
use fire_mission_planning::battery::GunFireSolution;
use fire_mission_planning::fire_mission::{
    FireMissionGunPosition, FireMissionInputs, FireMissionPoint, FireMissionSolution,
    FireMissionWind, HeightSource, solve_fire_mission,
};
use fire_mission_planning::solution_wording::{
    BatteryLineWords, ChargeRowWords, battery_line_words, charge_row_words, gun_heading, laid_rings,
};
use serde::Deserialize;

/// The tables of one solution as the page shows them.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct SolutionTables {
    /// The battery table: gun, charge, elevation, aim azimuth, time of flight.
    pub battery: Vec<Vec<String>>,
    /// One charge table per gun.
    pub guns: Vec<GunTable>,
}

/// One gun's charge table.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct GunTable {
    /// The heading, "<label> — <m> m · line <mils> mils · <deg>° · Δh <m> m".
    pub heading: String,
    /// Charge, elevation, aim azimuth, deflection, range correction, time of flight, apex.
    pub rows: Vec<Vec<String>>,
    /// Whether each row is the laid charge.
    pub laid: Vec<bool>,
}

/// The mission the gate typed, in map metres.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct TypedMission {
    /// Weapon.
    pub(super) weapon_id: String,
    /// Shell.
    pub(super) shell_id: String,
    /// Target position and height.
    pub(super) target: (f64, f64, f64),
    /// The gun's label, position and height.
    pub(super) gun: (String, f64, f64, f64),
    /// Wind speed (m/s) and the direction it blows from (degrees).
    pub(super) wind: (f64, f64),
}

/// Solves `mission` against `catalog` natively, with the recommended charge, no burst height and
/// no crest profile.
///
/// # Errors
///
/// The engine's refusal of the whole mission.
pub(super) fn solve_natively(
    catalog: &BallisticsCatalog,
    mission: &TypedMission,
) -> Result<FireMissionSolution> {
    let (tx, ty, th) = mission.target;
    let (label, gx, gy, gh) = mission.gun.clone();
    let inputs = FireMissionInputs {
        catalog_id: catalog.catalog_id.clone(),
        catalog_version: catalog.catalog_version,
        weapon_id: mission.weapon_id.clone().into(),
        shell_id: mission.shell_id.clone().into(),
        charge_rings: None,
        target: FireMissionPoint {
            x: tx,
            y: ty,
            height_m: th,
            height_source: HeightSource::Manual,
        },
        guns: vec![FireMissionGunPosition {
            label,
            x: gx,
            y: gy,
            height_m: gh,
            height_source: HeightSource::Manual,
        }],
        wind: Some(FireMissionWind {
            speed_m_s: mission.wind.0,
            from_deg: mission.wind.1.rem_euclid(360.0),
        }),
        burst_height_m: None,
        crest_profile: None,
    };
    solve_fire_mission(catalog, &inputs).map_err(|refusal| refusal!("native solve: {refusal}"))
}

/// The battery table row of `gun` laying its recommended charge.
fn battery_row(gun: &GunFireSolution) -> Vec<String> {
    let BatteryLineWords {
        label,
        charge,
        elevation,
        aim_azimuth,
        time_of_flight,
    } = battery_line_words(gun, None);
    vec![label, charge, elevation, aim_azimuth, time_of_flight]
}

/// The charge table of `gun` laying its recommended charge.
fn gun_table(gun: &GunFireSolution) -> GunTable {
    let rows = gun
        .charges
        .iter()
        .map(|c| {
            let ChargeRowWords {
                charge,
                elevation,
                aim_azimuth,
                deflection,
                range_correction,
                time_of_flight,
                apex,
            } = charge_row_words(c);
            vec![
                charge,
                elevation,
                aim_azimuth,
                deflection,
                range_correction,
                time_of_flight,
                apex,
            ]
        })
        .collect();
    let laid_charge = laid_rings(gun, None);
    let laid = gun
        .charges
        .iter()
        .map(|c| laid_charge == Some(c.rings))
        .collect();
    GunTable {
        heading: gun_heading(gun),
        rows,
        laid,
    }
}

/// The tables the page shows for `solution` with the recommended charge.
#[must_use]
pub(super) fn expected_tables(solution: &FireMissionSolution) -> SolutionTables {
    SolutionTables {
        battery: solution.guns.iter().map(battery_row).collect(),
        guns: solution.guns.iter().map(gun_table).collect(),
    }
}

/// Every difference between the page's tables and the expected ones; empty when they match.
#[must_use]
pub(super) fn table_differences(page: &SolutionTables, expected: &SolutionTables) -> Vec<String> {
    let mut out = Vec::new();
    if page.battery != expected.battery {
        out.push(format!(
            "battery: page {:?} != native {:?}",
            page.battery, expected.battery
        ));
    }
    if page.guns.len() != expected.guns.len() {
        out.push(format!(
            "gun tables: page {} != native {}",
            page.guns.len(),
            expected.guns.len()
        ));
    }
    for (i, (p, e)) in page.guns.iter().zip(&expected.guns).enumerate() {
        if p.heading != e.heading {
            out.push(format!(
                "gun {i} heading: page {:?} != native {:?}",
                p.heading, e.heading
            ));
        }
        if p.rows != e.rows {
            out.push(format!(
                "gun {i} rows: page {:?} != native {:?}",
                p.rows, e.rows
            ));
        }
        if p.laid != e.laid {
            out.push(format!(
                "gun {i} laid charge: page {:?} != native {:?}",
                p.laid, e.laid
            ));
        }
    }
    out
}

#[cfg(test)]
#[path = "../tests/mortar_offline/expected_solution.rs"]
mod tests;
