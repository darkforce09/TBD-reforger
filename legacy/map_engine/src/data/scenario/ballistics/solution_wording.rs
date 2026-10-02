//! The words a fire-mission solution is shown in: angles, corrections, times, apexes, refusals,
//! a gun's heading, its charge rows and its battery line.
//!
//! **Role:** turns a [`GunFireSolution`] and its [`ChargeSolution`] rows into the exact strings
//! an operator reads, so every surface that shows or checks a solution words it one way.
//!
//! **Position:** `data/scenario/ballistics`, after
//! [`crate::data::scenario::ballistics::fire_mission::solve_fire_mission`]; the mortar
//! calculator's solution panel renders these strings, and the offline browser gate compares the
//! page's tables with a native solve worded by these same functions.
//!
//! **Signals & state:** none; pure functions over plain values.
//!
//! **Invariants:**
//! - Every figure is rounded to the nearest value at its shown precision, from the exact binary
//!   value of the `f64` (Rust's formatter; an exact decimal tie rounds to the even digit):
//!   angles at 0.1 mil and 0.01°, corrections at 0.1, times of flight at 0.1 s, apexes and
//!   distances at 1 m, height differences at 0.1 m. No figure is truncated.
//! - A correction smaller than half its last shown digit (0.05) reads as zero with no side or
//!   sign, so a shown correction never rounds to a signed zero.
//! - A refused charge shows its refusal and no figures; a charge row missing any figure is
//!   worded as refused.
//! - The laid charge is the pinned charge when the operator chose one, else the gun's
//!   recommendation.

use crate::data::scenario::ballistics::battery::GunFireSolution;
use crate::data::scenario::ballistics::solver::{ChargeSolution, SolutionRefusal};

/// The words for a refused charge.
#[must_use]
pub fn refusal_text(refusal: SolutionRefusal) -> &'static str {
    match refusal {
        SolutionRefusal::TooClose => "too close for this charge",
        SolutionRefusal::OutOfRange => "out of range",
        SolutionRefusal::Unreachable => "target above the flight's apex",
        SolutionRefusal::DidNotConverge => "no stable solution found",
        SolutionRefusal::TimeToLiveExceeded => "shell expires before impact",
        SolutionRefusal::InvalidInput => "invalid input",
    }
}

/// The words for a charge with no refusal and no figures.
pub const NO_SOLUTION_TEXT: &str = "no solution";

/// An angle in the weapon's mils and in degrees, e.g. "1066.7 mils · 60.00°".
#[must_use]
pub fn mils_and_degrees(mils: f64, degrees: f64) -> String {
    format!("{mils:.1} mils · {degrees:.2}°")
}

/// A deflection correction: the side to lay off the target line and its size, e.g.
/// "R 3.2 mils"; "0.0 mils" below 0.05.
#[must_use]
pub fn deflection_text(mils: f64) -> String {
    if mils.abs() < 0.05 {
        "0.0 mils".to_string()
    } else if mils > 0.0 {
        format!("R {mils:.1} mils")
    } else {
        format!("L {:.1} mils", -mils)
    }
}

/// A range correction with its sign, e.g. "+12.3 m"; "0.0 m" below 0.05.
#[must_use]
pub fn range_correction_text(metres: f64) -> String {
    if metres.abs() < 0.05 {
        "0.0 m".to_string()
    } else {
        format!("{metres:+.1} m")
    }
}

/// A time of flight, e.g. "24.3 s".
#[must_use]
pub fn time_of_flight_text(seconds: f64) -> String {
    format!("{seconds:.1} s")
}

/// An apex above the gun, e.g. "734 m".
#[must_use]
pub fn apex_text(metres: f64) -> String {
    format!("{metres:.0} m")
}

/// A charge's name, e.g. "Charge 2".
#[must_use]
pub fn charge_label(rings: u32) -> String {
    format!("Charge {rings}")
}

/// The rings a gun lays: the mission's pinned `charge_rings`, else the gun's recommendation.
#[must_use]
pub fn laid_rings(gun: &GunFireSolution, charge_rings: Option<u32>) -> Option<u32> {
    charge_rings.or(gun.recommended_rings)
}

/// The heading of one gun's solution, e.g.
/// "Gun 1 — 985 m · line 1066.7 mils · 60.00° · Δh +12.0 m".
#[must_use]
pub fn gun_heading(gun: &GunFireSolution) -> String {
    format!(
        "{} — {:.0} m · line {} · Δh {:+.1} m",
        gun.label,
        gun.distance_m,
        mils_and_degrees(gun.azimuth_mils, gun.azimuth_deg),
        gun.height_difference_m
    )
}

/// One charge row in words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChargeRowWords {
    /// "Charge n".
    pub charge: String,
    /// Elevation, or the refusal.
    pub elevation: String,
    /// Aim azimuth to lay; empty when refused.
    pub aim_azimuth: String,
    /// Deflection correction; empty when refused.
    pub deflection: String,
    /// Range correction; empty when refused.
    pub range_correction: String,
    /// Time of flight; empty when refused.
    pub time_of_flight: String,
    /// Apex above the gun; empty when refused.
    pub apex: String,
}

/// The figures of a charge that solved, worded; `None` when refused or missing a figure.
fn solved_row_words(row: &ChargeSolution) -> Option<[String; 6]> {
    if row.refusal.is_some() {
        return None;
    }
    Some([
        mils_and_degrees(row.elevation_mils?, row.elevation_deg?),
        mils_and_degrees(row.aim_azimuth_mils?, row.aim_azimuth_deg?),
        deflection_text(row.deflection_correction_mils?),
        range_correction_text(row.range_correction_m?),
        time_of_flight_text(row.time_of_flight_s?),
        apex_text(row.apex_m?),
    ])
}

/// The words of one charge row.
#[must_use]
pub fn charge_row_words(row: &ChargeSolution) -> ChargeRowWords {
    let [
        elevation,
        aim_azimuth,
        deflection,
        range_correction,
        time_of_flight,
        apex,
    ] = solved_row_words(row).unwrap_or_else(|| {
        [
            row.refusal
                .map_or(NO_SOLUTION_TEXT, refusal_text)
                .to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ]
    });
    ChargeRowWords {
        charge: charge_label(row.rings),
        elevation,
        aim_azimuth,
        deflection,
        range_correction,
        time_of_flight,
        apex,
    }
}

/// One gun's battery line in words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatteryLineWords {
    /// The gun's label.
    pub label: String,
    /// "Charge n", or "No charge solves".
    pub charge: String,
    /// Elevation, or why the laid charge does not solve.
    pub elevation: String,
    /// Aim azimuth; empty when not solved.
    pub aim_azimuth: String,
    /// Time of flight; empty when not solved.
    pub time_of_flight: String,
}

/// The battery line of `gun` laying the mission's `charge_rings` (see [`laid_rings`]).
#[must_use]
pub fn battery_line_words(gun: &GunFireSolution, charge_rings: Option<u32>) -> BatteryLineWords {
    let label = gun.label.clone();
    let Some(rings) = laid_rings(gun, charge_rings) else {
        return BatteryLineWords {
            label,
            charge: "No charge solves".to_string(),
            elevation: String::new(),
            aim_azimuth: String::new(),
            time_of_flight: String::new(),
        };
    };
    let row = gun.charges.iter().find(|c| c.rings == rings);
    let charge = charge_label(rings);
    match row.and_then(solved_row_words) {
        Some([elevation, aim_azimuth, _, _, time_of_flight, _]) => BatteryLineWords {
            label,
            charge,
            elevation,
            aim_azimuth,
            time_of_flight,
        },
        None => BatteryLineWords {
            label,
            charge,
            elevation: row
                .and_then(|c| c.refusal)
                .map_or(NO_SOLUTION_TEXT, refusal_text)
                .to_string(),
            aim_azimuth: String::new(),
            time_of_flight: String::new(),
        },
    }
}

#[cfg(test)]
#[path = "tests/solution_wording.rs"]
mod tests;
