//! Calibration of a ballistics catalog against the game's own tables and engine oracle samples.
//!
//! **Role:** decodes a calibration bundle and judges it against the catalog it pins:
//! [`evaluate`] answers a [`CalibrationReport`] listing every provenance mismatch and every
//! native-table row, wind-table row, forward-angle sample and simulation sample the flight model
//! misses beyond tolerance.
//!
//! **Position:** `data/scenario/ballistics`; the catalog upload of the API and the map engine's
//! calibration tests run the same [`evaluate`]. It reads the catalog through
//! [`crate::data::scenario::ballistics::catalog`] and flies through
//! [`crate::data::scenario::ballistics::flight_model`].
//!
//! **Signals & state:** none; pure functions over decoded documents.
//!
//! **Invariants:**
//! - Field names and shapes match `contracts_v2/definitions/ballistics-calibration.schema.json`;
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

pub mod charge_flight;
pub mod native_tables;
pub mod oracle_samples;
pub mod provenance;
pub mod report;
mod sha256_digest;
pub mod wind_tables;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::catalog::{BallisticsCatalog, CatalogDecodeError, CatalogResource};

/// Re-exports the bundle parts, the report and the failure classes.
pub use native_tables::{NativeTable, NativeTableRow};
pub use oracle_samples::{OracleSample, OracleSampleKind};
pub use report::{CalibrationFailure, CalibrationReport, FailureKind};
pub use wind_tables::{WindTable, WindTableRow};

/// The only calibration bundle version this crate reads.
pub const CALIBRATION_SCHEMA_VERSION: u32 = 1;

/// Every calibration case for one catalog version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationBundle {
    /// Document version; [`CALIBRATION_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Catalog the bundle calibrates.
    pub catalog_id: String,
    /// Version of that catalog.
    pub catalog_version: u32,
    /// Lowercase hexadecimal SHA-256 of the exact catalog bytes.
    pub catalog_sha256: String,
    /// Dotted game build the tables and samples come from.
    pub game_build: String,
    /// Enfusion GUID of the equipment export generation.
    pub export_generation_id: String,
    /// Every game resource the tables were read from.
    pub resources: Vec<CatalogResource>,
    /// The oracle run the samples and the catalog's gravity come from.
    pub oracle_run: OracleRun,
    /// The game's ballistic tables.
    pub native_tables: Vec<NativeTable>,
    /// The game's wind tables.
    pub wind_tables: Vec<WindTable>,
    /// The engine oracle's samples.
    pub oracle_samples: Vec<OracleSample>,
}

/// The Workbench oracle plugin run behind a bundle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OracleRun {
    /// Revision of the oracle plugin.
    pub plugin_revision: String,
    /// Gravity the physics world reported, metres per second squared.
    pub gravity_reported_m_s2: f64,
    /// RFC 3339 time the run started.
    pub run_at: String,
    /// SHA-256 of the raw oracle output the samples were trimmed from.
    pub output_sha256: String,
}

/// Why bytes are not a readable calibration bundle.
#[derive(Debug, Error)]
pub enum CalibrationDecodeError {
    /// The bytes are not JSON of the bundle's shape.
    #[error("calibration JSON does not decode: {0}")]
    Json(#[from] serde_json::Error),
    /// The document declares a version this crate does not read.
    #[error(
        "calibration schema_version {0} is not supported (expected {CALIBRATION_SCHEMA_VERSION})"
    )]
    UnsupportedSchemaVersion(u32),
}

impl CalibrationBundle {
    /// Decodes one calibration bundle.
    ///
    /// # Errors
    ///
    /// [`CalibrationDecodeError::Json`] for malformed JSON, a missing or unknown field or a value
    /// of the wrong type; [`CalibrationDecodeError::UnsupportedSchemaVersion`] for any version
    /// but [`CALIBRATION_SCHEMA_VERSION`].
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, CalibrationDecodeError> {
        let bundle: Self = serde_json::from_slice(bytes)?;
        if bundle.schema_version != CALIBRATION_SCHEMA_VERSION {
            return Err(CalibrationDecodeError::UnsupportedSchemaVersion(
                bundle.schema_version,
            ));
        }
        Ok(bundle)
    }
}

/// A decoded catalog with the SHA-256 of the exact bytes it was decoded from.
#[derive(Debug, Clone, PartialEq)]
pub struct PinnedCatalog {
    /// The decoded catalog.
    pub catalog: BallisticsCatalog,
    /// Lowercase hexadecimal SHA-256 of the catalog bytes.
    pub sha256: String,
}

impl PinnedCatalog {
    /// Decodes `bytes` as a catalog and hashes them.
    ///
    /// # Errors
    ///
    /// The [`CatalogDecodeError`] of [`BallisticsCatalog::from_json_slice`].
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, CatalogDecodeError> {
        Ok(Self {
            catalog: BallisticsCatalog::from_json_slice(bytes)?,
            sha256: sha256_digest::sha256_hex(bytes),
        })
    }
}

/// Judges `bundle` against `pinned`: provenance first, then every shell's cases in catalog order.
pub fn evaluate(pinned: &PinnedCatalog, bundle: &CalibrationBundle) -> CalibrationReport {
    let mut report = provenance::check_provenance(pinned, bundle);
    for shell in &pinned.catalog.shells {
        report.absorb(evaluate_shell(pinned, bundle, &shell.shell_id));
    }
    report
}

/// Judges the native rows, oracle samples and wind rows of one catalog shell; provenance is not
/// judged here.
pub fn evaluate_shell(
    pinned: &PinnedCatalog,
    bundle: &CalibrationBundle,
    shell_id: &str,
) -> CalibrationReport {
    let mut report = CalibrationReport::default();
    let catalog = &pinned.catalog;
    let Some(shell) = catalog
        .shells
        .iter()
        .find(|shell| shell.shell_id == shell_id)
    else {
        report.fail(
            FailureKind::UnknownShell,
            format!("coverage/{shell_id}"),
            format!("the catalog declares no shell {shell_id}"),
        );
        return report;
    };
    let mut flights = charge_flight::ChargeFlights::new(catalog.gravity_m_s2, shell);
    for table in bundle
        .native_tables
        .iter()
        .filter(|table| table.shell_id == shell_id)
    {
        native_tables::judge_native_table(&mut report, &mut flights, table);
    }
    for (index, sample) in bundle.oracle_samples.iter().enumerate() {
        if sample.shell_id == shell_id {
            oracle_samples::judge_oracle_sample(
                &mut report,
                &mut flights,
                &bundle.native_tables,
                index,
                sample,
            );
        }
    }
    for table in bundle
        .wind_tables
        .iter()
        .filter(|table| table.shell_id == shell_id)
    {
        wind_tables::judge_wind_table(&mut report, &mut flights, table);
    }
    report
}

#[cfg(test)]
#[path = "tests/committed_bundle.rs"]
mod tests;
