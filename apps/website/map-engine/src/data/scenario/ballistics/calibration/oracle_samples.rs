//! Engine oracle samples and their judgement against the flight model.
//!
//! **Role:** the serde form of one oracle call, the typed decode of the calls the calibration
//! judges, and their judges: forward-angle samples as native rows, simulation samples by their
//! point of fall and time of flight.
//!
//! **Position:** `ballistics/calibration`; decoded inside [`super::CalibrationBundle`] and judged
//! by [`super::evaluate_shell`].
//!
//! **Signals & state:** none; plain data and pure judges.
//!
//! **Invariants:**
//! - `inputs` and `outputs` keep the oracle's names and values verbatim; a judged sample missing
//!   a value its kind needs is an [`FailureKind::UnreadableCase`] failure, never a pass.
//! - The engine answers a forward-angle call between native rows by linear interpolation of its
//!   own table, so only a forward-angle sample at a native row of the same shell and coefficient
//!   (see [`NativeTable::has_row_at`]) is a case: it passes iff its range lies within the model's
//!   calm range over its `elevation_rad` ± 1 mil (6400) and its time of flight is within 0.1 s
//!   of the model's. Every other one is counted as engine table interpolation, not judged.
//! - A simulation sample flies the model at its elevation, azimuth, wind (speed and "from"
//!   direction, the engine's wind vector being `-speed · (sin from, 0, cos from)` in Enfusion's
//!   x-east, y-up, z-north frame) and target height; it passes iff the downrange and the
//!   crossrange differences are each within `D · tan(1 mil)`, `D` the oracle's horizontal
//!   distance to the point of fall, and the time of flight is within 0.1 s. A sample the engine
//!   reports as never reaching its height passes iff the model refuses the flight too.
//! - Altitude-difference samples are carried raw and judged by no criterion: the engine answered
//!   `false` to every committed call, so their decoding is not established.
//!
//! @contract ballistics-calibration.schema.json#/definitions/OracleSample

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::charge_flight::{
    ChargeFlights, ElevationCase, ONE_MIL_6400_RAD, TIME_OF_FLIGHT_TOLERANCE_S,
    judge_elevation_case,
};
use super::native_tables::NativeTable;
use super::report::{CalibrationReport, FailureKind};
use crate::data::scenario::ballistics::wind::Wind;

/// Which engine call a sample records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleSampleKind {
    /// `BallisticTable.GetDistanceOfProjectileSource`: range and time of flight at an elevation.
    ForwardAngle,
    /// `ProjectileMoveComponent.GetProjectileSimulationResult`: a full launch with wind and
    /// target height.
    Simulation,
    /// `GetAimHeightOfProjectileAltitudeFromSource`: aim and time for a height difference.
    AltitudeDifference,
}

/// One engine oracle call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OracleSample {
    /// The engine call.
    pub kind: OracleSampleKind,
    /// Catalog shell the call fired.
    pub shell_id: String,
    /// Muzzle speed coefficient of the charge.
    pub init_speed_coef: f64,
    /// The call's arguments under the oracle's names.
    pub inputs: Value,
    /// The call's results under the oracle's names.
    pub outputs: Value,
}

/// The inputs a forward-angle sample is judged on.
#[derive(Debug, Clone, Copy, Deserialize)]
struct ForwardAngleInputs {
    elevation_rad: f64,
}

/// The outputs a forward-angle sample is judged on.
#[derive(Debug, Clone, Copy, Deserialize)]
struct ForwardAngleOutputs {
    range_m: f64,
    time_of_flight_s: f64,
}

/// The inputs a simulation sample is judged on.
#[derive(Debug, Clone, Copy, Deserialize)]
struct SimulationInputs {
    elevation_deg: f64,
    azimuth_deg: f64,
    wind_speed_m_s: f64,
    wind_from_deg: f64,
    target_height_m: f64,
}

/// The outputs a simulation sample is judged on.
#[derive(Debug, Clone, Copy, Deserialize)]
struct SimulationOutputs {
    downrange_m: f64,
    crossrange_m: f64,
    time_of_flight_s: f64,
    reached_target_height: bool,
}

/// Judges one sample; `index` is its position in the bundle and `native_tables` are the
/// bundle's native tables, which decide whether a forward-angle sample is a case.
pub fn judge_oracle_sample(
    report: &mut CalibrationReport,
    flights: &mut ChargeFlights<'_>,
    native_tables: &[NativeTable],
    index: usize,
    sample: &OracleSample,
) {
    match sample.kind {
        OracleSampleKind::ForwardAngle => {
            judge_forward_angle(report, flights, native_tables, index, sample);
        }
        OracleSampleKind::Simulation => judge_simulation(report, flights, index, sample),
        OracleSampleKind::AltitudeDifference => {}
    }
}

fn judge_forward_angle(
    report: &mut CalibrationReport,
    flights: &mut ChargeFlights<'_>,
    native_tables: &[NativeTable],
    index: usize,
    sample: &OracleSample,
) {
    let case_id = format!(
        "forward/{}/{}/{index}",
        sample.shell_id, sample.init_speed_coef
    );
    let decoded = ForwardAngleInputs::deserialize(&sample.inputs)
        .and_then(|inputs| Ok((inputs, ForwardAngleOutputs::deserialize(&sample.outputs)?)));
    let (inputs, outputs) = match decoded {
        Ok(decoded) => decoded,
        Err(error) => {
            report.cases += 1;
            return unreadable(report, case_id, &error);
        }
    };
    let at_native_row = native_tables.iter().any(|table| {
        table.has_row_at(
            &sample.shell_id,
            sample.init_speed_coef,
            inputs.elevation_rad,
        )
    });
    if !at_native_row {
        report.interpolated_forward_samples += 1;
        return;
    }
    let case = ElevationCase {
        elevation_rad: inputs.elevation_rad,
        range_m: outputs.range_m,
        time_of_flight_s: Some(outputs.time_of_flight_s),
        range_failure: FailureKind::ForwardSampleRange,
        time_failure: FailureKind::ForwardSampleTimeOfFlight,
    };
    judge_elevation_case(report, flights.at(sample.init_speed_coef), &case_id, &case);
}

fn judge_simulation(
    report: &mut CalibrationReport,
    flights: &mut ChargeFlights<'_>,
    index: usize,
    sample: &OracleSample,
) {
    let case_id = format!(
        "simulation/{}/{}/{index}",
        sample.shell_id, sample.init_speed_coef
    );
    report.cases += 1;
    let decoded = SimulationInputs::deserialize(&sample.inputs)
        .and_then(|inputs| Ok((inputs, SimulationOutputs::deserialize(&sample.outputs)?)));
    let (inputs, outputs) = match decoded {
        Ok(decoded) => decoded,
        Err(error) => return unreadable(report, case_id, &error),
    };
    let wind = Wind {
        speed_m_s: inputs.wind_speed_m_s,
        from_deg: inputs.wind_from_deg,
    };
    let flown = flights.at(sample.init_speed_coef).fly(
        inputs.elevation_deg.to_radians(),
        inputs.azimuth_deg.to_radians(),
        &wind,
        inputs.target_height_m,
    );
    let outcome = match (flown, outputs.reached_target_height) {
        (Ok(outcome), true) => outcome,
        (Err(_), false) => return,
        (Ok(outcome), false) => {
            return report.fail(
                FailureKind::SimulationImpact,
                case_id,
                format!(
                    "the engine never reaches height {} m but the model falls through it at {:.3} m",
                    inputs.target_height_m, outcome.downrange_m
                ),
            );
        }
        (Err(error), true) => {
            return report.fail(
                FailureKind::ModelRefused,
                case_id,
                format!("the flight model refused the case: {error}"),
            );
        }
    };
    let distance_m = libm::hypot(outputs.downrange_m, outputs.crossrange_m);
    let tolerance_m = distance_m * libm::tan(ONE_MIL_6400_RAD);
    let downrange_error_m = outcome.downrange_m - outputs.downrange_m;
    let crossrange_error_m = outcome.deflection_m - outputs.crossrange_m;
    if downrange_error_m.abs() > tolerance_m || crossrange_error_m.abs() > tolerance_m {
        report.fail(
            FailureKind::SimulationImpact,
            case_id.clone(),
            format!(
                "point of fall ({}, {}) m differs from the model's ({:.3}, {:.3}) m by ({downrange_error_m:.3}, {crossrange_error_m:.3}) m, beyond {tolerance_m:.3} m",
                outputs.downrange_m, outputs.crossrange_m, outcome.downrange_m, outcome.deflection_m
            ),
        );
    }
    if (outcome.time_of_flight_s - outputs.time_of_flight_s).abs() > TIME_OF_FLIGHT_TOLERANCE_S {
        report.fail(
            FailureKind::SimulationTimeOfFlight,
            case_id,
            format!(
                "time of flight {} s differs from the model's {:.4} s by more than {TIME_OF_FLIGHT_TOLERANCE_S} s",
                outputs.time_of_flight_s, outcome.time_of_flight_s
            ),
        );
    }
}

/// Records a sample missing a value its kind needs.
fn unreadable(report: &mut CalibrationReport, case_id: String, error: &serde_json::Error) {
    report.fail(
        FailureKind::UnreadableCase,
        case_id,
        format!("the sample does not carry the values its kind needs: {error}"),
    );
}
