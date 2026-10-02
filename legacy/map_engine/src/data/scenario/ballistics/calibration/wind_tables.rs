//! The game's wind tables and their judgement against the flight model.
//!
//! **Role:** the serde form of a wind table (one shell at one muzzle speed coefficient and one
//! wind speed), the decode of its per-row value list, and the judge of each row.
//!
//! **Position:** `ballistics/calibration`; decoded inside [`super::CalibrationBundle`] and judged
//! by [`super::evaluate_shell`].
//!
//! **Signals & state:** none; plain data and one pure judge.
//!
//! **Invariants:**
//! - A row's `(elevation_rad, range_m)` is a calm firing solution: it passes iff `range_m` lies
//!   within the model's calm range over the elevation ± 1 mil (6400). `apex_m` is not judged.
//! - `values` decodes as `[crosswind deflection, range change, angle of fall]` (see the module
//!   README for the evidence): `values[0]` is `1000 · atan(deflection / downrange)` in
//!   milliradians, both measured at the point of fall under a crosswind of the table's speed
//!   (the downrange under that crosswind, not the calm range); `values[1]` is the range change
//!   in metres under a wind of the table's speed along the line of fire, half the difference
//!   between the tail-wind and the head-wind ranges; `values[2]` is the angle of fall in degrees
//!   and is not judged.
//! - `elevation_rad` is stated to three decimals (up to half a mil off the row's elevation), the
//!   calm range to the millimetre: the wind effects are flown at the elevation inside the row's
//!   ± 1 mil window where the model's calm range equals `range_m`
//!   ([`ChargeFlight::calm_elevation_for_range`]), or at the stated elevation when the window
//!   holds none (the range judge then already fails the row).
//! - The crosswind passes iff the model's `atan(deflection / downrange)` under the same
//!   crosswind lies within 1 mil (6400) of `values[0] / 1000`; the range change passes iff the
//!   model's lies within `(range_m + |values[1]|) · tan(1 mil)` of `values[1]`. A row at zero
//!   range carries no deflection magnitude (its angle is `π/2` whatever the drift), so its
//!   crosswind is not judged.
//!
//! @contract ballistics-calibration.schema.json#/definitions/WindTable
//! @contract ballistics-calibration.schema.json#/definitions/WindTableRow

use serde::{Deserialize, Serialize};

use super::charge_flight::{
    ChargeFlight, ChargeFlights, ElevationCase, ONE_MIL_6400_RAD, judge_elevation_case, refuse,
};
use super::report::{CalibrationReport, FailureKind};
use crate::data::scenario::ballistics::flight_model::FlightOutcome;
use crate::data::scenario::ballistics::wind::Wind;

/// Milliradians per radian: the unit of the crosswind deflection value.
const MILLIRADIANS_PER_RADIAN: f64 = 1000.0;

/// One game wind table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindTable {
    /// Catalog shell the table belongs to.
    pub shell_id: String,
    /// Muzzle speed coefficient of the charge.
    pub init_speed_coef: f64,
    /// Wind speed the table's wind effects are computed at, metres per second.
    pub wind_speed_m_s: f64,
    /// The table's rows in game order.
    pub rows: Vec<WindTableRow>,
}

/// One row of a wind table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindTableRow {
    /// Elevation in radians.
    pub elevation_rad: f64,
    /// Calm range in metres at the muzzle height.
    pub range_m: f64,
    /// Calm apex height in metres above the muzzle.
    pub apex_m: f64,
    /// The game's value list, verbatim.
    pub values: Vec<f64>,
}

/// The decoded wind effects of one row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindRowEffects {
    /// Crosswind deflection angle, milliradians.
    pub crosswind_deflection_mrad: f64,
    /// Range change under a wind along the line of fire, metres.
    pub range_change_m: f64,
}

impl WindTableRow {
    /// The decoded wind effects, or `None` when the value list is too short.
    pub fn effects(&self) -> Option<WindRowEffects> {
        match self.values.as_slice() {
            [deflection, range_change, ..] => Some(WindRowEffects {
                crosswind_deflection_mrad: *deflection,
                range_change_m: *range_change,
            }),
            _ => None,
        }
    }
}

/// Judges every row of `table` with the flights of its shell.
pub fn judge_wind_table(
    report: &mut CalibrationReport,
    flights: &mut ChargeFlights<'_>,
    table: &WindTable,
) {
    let flight = flights.at(table.init_speed_coef);
    for (index, row) in table.rows.iter().enumerate() {
        let case_id = format!(
            "wind/{}/{}/{}/{index}",
            table.shell_id, table.init_speed_coef, table.wind_speed_m_s
        );
        let case = ElevationCase {
            elevation_rad: row.elevation_rad,
            range_m: row.range_m,
            time_of_flight_s: None,
            range_failure: FailureKind::WindRowRange,
            time_failure: FailureKind::WindRowRange,
        };
        judge_elevation_case(report, flight, &case_id, &case);
        let Some(effects) = row.effects() else {
            report.fail(
                FailureKind::UnreadableCase,
                case_id,
                format!("wind row carries {} values, fewer than 2", row.values.len()),
            );
            continue;
        };
        judge_wind_effects(report, flight, &case_id, table.wind_speed_m_s, row, effects);
    }
}

/// Judges the crosswind deflection and the range change of one row.
fn judge_wind_effects(
    report: &mut CalibrationReport,
    flight: &ChargeFlight,
    case_id: &str,
    wind_speed_m_s: f64,
    row: &WindTableRow,
    effects: WindRowEffects,
) {
    let wind_from = |from_deg: f64| Wind {
        speed_m_s: wind_speed_m_s,
        from_deg,
    };
    let tolerance_per_metre = libm::tan(ONE_MIL_6400_RAD);
    let elevation_rad = flight
        .calm_elevation_for_range(row.elevation_rad, row.range_m)
        .unwrap_or(row.elevation_rad);
    if row.range_m > 0.0 {
        let expected_rad = effects.crosswind_deflection_mrad / MILLIRADIANS_PER_RADIAN;
        // A wind from the west blows east, to the right of a northward line of fire.
        match flight.fly(elevation_rad, 0.0, &wind_from(270.0), 0.0) {
            Ok(outcome) => {
                let modelled_rad = crosswind_deflection_angle_rad(&outcome);
                if (modelled_rad - expected_rad).abs() > ONE_MIL_6400_RAD {
                    report.fail(
                        FailureKind::WindRowCrosswind,
                        case_id.to_owned(),
                        format!(
                            "crosswind deflection {} mrad differs from the model's {:.3} mrad by more than 1 mil",
                            effects.crosswind_deflection_mrad,
                            modelled_rad * MILLIRADIANS_PER_RADIAN
                        ),
                    );
                }
            }
            Err(error) => refuse(report, case_id, &error),
        }
    }
    let head = flight.fly(elevation_rad, 0.0, &wind_from(0.0), 0.0);
    let tail = flight.fly(elevation_rad, 0.0, &wind_from(180.0), 0.0);
    match (head, tail) {
        (Ok(head), Ok(tail)) => {
            let modelled_m = (tail.downrange_m - head.downrange_m) / 2.0;
            let tolerance_m =
                (row.range_m.abs() + effects.range_change_m.abs()) * tolerance_per_metre;
            if (modelled_m - effects.range_change_m).abs() > tolerance_m {
                report.fail(
                    FailureKind::WindRowRangeWind,
                    case_id.to_owned(),
                    format!(
                        "range change {} m under wind along the line of fire differs from the model's {modelled_m:.3} m by more than {tolerance_m:.3} m",
                        effects.range_change_m
                    ),
                );
            }
        }
        (Err(error), _) | (_, Err(error)) => refuse(report, case_id, &error),
    }
}

/// The deflection angle of a point of fall as the wind table states it,
/// `atan(deflection / downrange)`, radians.
pub fn crosswind_deflection_angle_rad(outcome: &FlightOutcome) -> f64 {
    libm::atan(outcome.deflection_m / outcome.downrange_m)
}
