//! The game's own ballistic tables and their judgement against the flight model.
//!
//! **Role:** the serde form of a native table (one shell at one muzzle speed coefficient) and the
//! judge of each of its rows.
//!
//! **Position:** the `ballistics_calibration` crate; decoded inside [`super::CalibrationBundle`] and judged
//! by [`super::evaluate_shell`] through [`super::charge_flight::judge_elevation_case`].
//!
//! **Signals & state:** none; plain data and one pure judge.
//!
//! **Invariants:**
//! - A row passes iff its range lies within the model's range over its elevation ± 1 mil (6400)
//!   and the model's time of flight at its elevation is within 0.1 s of the row's.
//! - `column_1` is carried verbatim and judged by no criterion; on every committed row it equals
//!   `range_m · tan(elevation)`, the height of the line of departure above the point of fall (see
//!   the module README).
//!
//! - A row lies at an elevation when the two differ by at most
//!   [`ROW_ELEVATION_MATCH_TOLERANCE_RAD`]: far above the `f32` rounding of an oracle elevation
//!   (under 1.2e-7 rad below π/2) and far below the spacing of the oracle's elevations between
//!   rows (the nearest lies 1.5 mil, 1.5e-3 rad, from a row).
//!
//! @contract ballistics-calibration.schema.json#/definitions/NativeTable
//! @contract ballistics-calibration.schema.json#/definitions/NativeTableRow

use serde::{Deserialize, Serialize};

use super::charge_flight::{
    ChargeFlights, ElevationCase, ONE_MIL_6400_RAD, judge_elevation_case, same_coefficient,
};
use super::report::{CalibrationReport, FailureKind};
use crate::ids::CalibrationCaseId;
use ballistics_model::ids::ShellId;

/// Largest elevation difference, radians, at which an elevation names a native row (0.001 mil).
pub const ROW_ELEVATION_MATCH_TOLERANCE_RAD: f64 = 1e-6;

/// One game ballistic table: a shell at one muzzle speed coefficient.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeTable {
    /// Catalog shell the table belongs to.
    pub shell_id: ShellId,
    /// Muzzle speed coefficient of the charge.
    pub init_speed_coef: f64,
    /// The table's rows in game order.
    pub rows: Vec<NativeTableRow>,
}

/// One row of a native table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeTableRow {
    /// Position of the row in the game's table.
    pub lattice_index: u32,
    /// Elevation of the row in mils of the 6400 convention, assigned from the oracle.
    pub elevation_mils_6400: f64,
    /// Range in metres at the muzzle height.
    pub range_m: f64,
    /// The game's second column, verbatim.
    pub column_1: f64,
    /// Time of flight in seconds.
    pub time_of_flight_s: f64,
}

impl NativeTable {
    /// Whether this table belongs to `shell_id` at `init_speed_coef` and carries a row at
    /// `elevation_rad` (within [`ROW_ELEVATION_MATCH_TOLERANCE_RAD`]).
    pub fn has_row_at(&self, shell_id: &ShellId, init_speed_coef: f64, elevation_rad: f64) -> bool {
        self.shell_id == *shell_id
            && same_coefficient(self.init_speed_coef, init_speed_coef)
            && self.rows.iter().any(|row| {
                (row.elevation_rad() - elevation_rad).abs() <= ROW_ELEVATION_MATCH_TOLERANCE_RAD
            })
    }
}

impl NativeTableRow {
    /// The row's elevation in radians.
    pub fn elevation_rad(&self) -> f64 {
        self.elevation_mils_6400 * ONE_MIL_6400_RAD
    }
}

/// Judges every row of `table` with the flights of its shell.
pub fn judge_native_table(
    report: &mut CalibrationReport,
    flights: &mut ChargeFlights<'_>,
    table: &NativeTable,
) {
    let flight = flights.at(table.init_speed_coef);
    for row in &table.rows {
        let case_id = CalibrationCaseId::from(format!(
            "native/{}/{}/{}",
            table.shell_id, table.init_speed_coef, row.lattice_index
        ));
        let case = ElevationCase {
            elevation_rad: row.elevation_rad(),
            range_m: row.range_m,
            time_of_flight_s: Some(row.time_of_flight_s),
            range_failure: FailureKind::NativeRowRange,
            time_failure: FailureKind::NativeRowTimeOfFlight,
        };
        judge_elevation_case(report, flight, &case_id, &case);
    }
}
