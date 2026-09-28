//! The outcome of one calibration run: how many cases were judged and every failure.
//!
//! **Role:** the typed report [`super::evaluate`] returns and its wire projection: the `cases`,
//! `failures` and `forward_samples_not_judged` of a catalog upload report.
//!
//! **Position:** `ballistics/calibration`; the provenance, native-table, wind-table and
//! oracle-sample judges push [`CalibrationFailure`]s; the API turns a [`CalibrationReport`] into
//! its `CatalogUploadReport` and the tests assert on [`FailureKind`].
//!
//! **Signals & state:** none; plain data.
//!
//! **Invariants:**
//! - A report accepts its catalog exactly when it lists no failure.
//! - A forward-angle sample between native rows is engine table interpolation: it is counted in
//!   `interpolated_forward_samples` (on the wire `forward_samples_not_judged`), never in
//!   `cases`, and can neither pass nor fail.
//! - Every failure carries a stable `case_id` naming the case (`native/<shell>/<coef>/<lattice
//!   index>`, `forward/…`, `wind/…`, `simulation/…`, `provenance/…`, `coverage/…`) and a reason
//!   stating both the expected and the modelled values; [`FailureKind`] is never serialised.
//!
//! @contract ballistics-catalog.schema.json#/definitions/CatalogUploadReport
//! @contract ballistics-catalog.schema.json#/definitions/CalibrationFailure

use serde::Serialize;

/// Cases judged and the failures among them.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CalibrationReport {
    /// Calibration cases evaluated (provenance checks are not counted as cases).
    pub cases: u32,
    /// Every case outside tolerance and every provenance mismatch, in evaluation order.
    pub failures: Vec<CalibrationFailure>,
    /// Forward-angle samples between native rows: the engine's interpolation of its own table,
    /// not judged and not counted in [`CalibrationReport::cases`]; serialised as the upload
    /// report's `forward_samples_not_judged`.
    #[serde(rename = "forward_samples_not_judged")]
    pub interpolated_forward_samples: u32,
}

impl CalibrationReport {
    /// Whether the catalog passes: no failure at all.
    pub fn accepted(&self) -> bool {
        self.failures.is_empty()
    }

    /// Appends the cases and failures of `other`.
    pub fn absorb(&mut self, other: CalibrationReport) {
        self.cases += other.cases;
        self.failures.extend(other.failures);
        self.interpolated_forward_samples += other.interpolated_forward_samples;
    }

    /// Records one failure.
    pub(super) fn fail(&mut self, kind: FailureKind, case_id: String, reason: String) {
        self.failures.push(CalibrationFailure {
            case_id,
            reason,
            kind,
        });
    }
}

/// One failed case or provenance check.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CalibrationFailure {
    /// Stable identifier of the case or check.
    pub case_id: String,
    /// What differs: the expected and the modelled or declared values.
    pub reason: String,
    /// The typed class of the failure.
    #[serde(skip)]
    pub kind: FailureKind,
}

/// Every class of calibration failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureKind {
    /// The bundle names another catalog id or version.
    CatalogIdentityMismatch,
    /// The bundle's game build differs from the catalog's.
    GameBuildMismatch,
    /// The bundle's export generation differs from the catalog's.
    ExportGenerationMismatch,
    /// The bundle's `catalog_sha256` is not the SHA-256 of the catalog bytes.
    CatalogShaMismatch,
    /// The catalog's gravity differs from the gravity the oracle run reported.
    GravityMismatch,
    /// One resource GUID is declared with two different SHA-256 digests.
    ResourceShaMismatch,
    /// A (shell, charge) of the catalog has no native table.
    MissingNativeTable,
    /// A (shell, charge) of the catalog has no simulation sample.
    MissingSimulationSample,
    /// A table or sample names a shell the catalog does not declare.
    UnknownShell,
    /// A table row, wind row or sample does not carry the values its kind needs.
    UnreadableCase,
    /// The flight model refused a flight the case needs.
    ModelRefused,
    /// A native-table row's range lies outside the model's range over its elevation ± 1 mil.
    NativeRowRange,
    /// A native-table row's time of flight differs from the model's by more than 0.1 s.
    NativeRowTimeOfFlight,
    /// A forward-angle sample's range lies outside the model's range over its elevation ± 1 mil.
    ForwardSampleRange,
    /// A forward-angle sample's time of flight differs from the model's by more than 0.1 s.
    ForwardSampleTimeOfFlight,
    /// A wind-table row's range lies outside the model's range over its elevation ± 1 mil.
    WindRowRange,
    /// A wind-table row's crosswind deflection differs from the model's beyond tolerance.
    WindRowCrosswind,
    /// A wind-table row's range change under a head or tail wind differs beyond tolerance.
    WindRowRangeWind,
    /// A simulation sample's impact point differs from the model's beyond tolerance.
    SimulationImpact,
    /// A simulation sample's time of flight differs from the model's by more than 0.1 s.
    SimulationTimeOfFlight,
}

#[cfg(test)]
#[path = "tests/report.rs"]
mod tests;
