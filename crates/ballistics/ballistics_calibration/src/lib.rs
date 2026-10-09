//! Calibration of a ballistics catalog against the game's own tables and engine oracle samples.
//!
//! **Role:** decodes a calibration bundle and judges it against the catalog it pins:
//! [`evaluate`] answers a [`CalibrationReport`] listing every provenance mismatch and every
//! native-table row, wind-table row, forward-angle sample and simulation sample the flight model
//! misses beyond tolerance.
//! **Position:** ballistics tier 3, over `ballistics_model` (the catalog and the flight model),
//! `ballistics_solver` (the elevation searches) and `content_digest` (the catalog bytes'
//! SHA-256). The API's catalog upload and this crate's tests run the same [`evaluate`].
//! **Signals & state:** none; pure functions over decoded documents.
//! **Invariants:**
//! - Field names and shapes match `contracts/definitions/ballistics-calibration.schema.json`;
//!   every object refuses unknown fields, except an oracle sample's `inputs` and `outputs`, which
//!   keep the oracle's own names.
//! - Provenance is judged first ([`provenance`]); flight cases follow shell by shell in catalog
//!   order ([`evaluate_shell`]). Every case's tolerance is fixed: 1 mil of the 6400 convention
//!   and 0.1 s ([`charge_flight`]). Nothing widens it.
//! - A report accepts its catalog exactly when it lists no failure.
//!
//! @contract ballistics-calibration.schema.json#/definitions/CalibrationBundle
//! @contract ballistics-calibration.schema.json#/definitions/CalibrationResource
//! @contract ballistics-calibration.schema.json#/definitions/OracleRun

mod calibration_bundle;
pub mod charge_flight;
mod error;
pub mod ids;
pub mod native_tables;
pub mod oracle_samples;
pub mod prelude;
pub mod provenance;
pub mod report;
pub mod wind_tables;

/// The bundle, its oracle run, its decode error, the pinned catalog and the two judges.
pub use calibration_bundle::{
    CALIBRATION_SCHEMA_VERSION, CalibrationBundle, CalibrationDecodeError, OracleRun,
    PinnedCatalog, evaluate, evaluate_shell,
};
/// The crate's error and result.
pub use error::{Error, Result};
/// The identifier of one judged calibration case.
pub use ids::CalibrationCaseId;
/// A native ballistic table and its rows.
pub use native_tables::{NativeTable, NativeTableRow};
/// An engine oracle sample and its kind.
pub use oracle_samples::{OracleSample, OracleSampleKind};
/// The report, its failures and the failure classes.
pub use report::{CalibrationFailure, CalibrationReport, FailureKind};
/// A wind table and its rows.
pub use wind_tables::{WindTable, WindTableRow};

/// The committed vanilla catalog and its bundle, judged whole.
#[cfg(test)]
#[path = "tests/committed_bundle.rs"]
mod tests;

/// The firing solver inverted over the engine oracle's height and wind samples.
#[cfg(test)]
#[path = "tests/oracle_elevation_and_wind.rs"]
mod tests_oracle_elevation_and_wind;
