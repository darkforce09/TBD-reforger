//! Ballistics catalogs: the stored versions, the catalog document, and the upload outcome.
//!
//! **Role:** the wire shapes of the ballistics-catalog routes — the public list of stored
//! versions ([`BallisticsCatalogList`] of [`BallisticsCatalogSummary`]), one version's catalog
//! document ([`ballistics_model::catalog::BallisticsCatalog`], the `ballistics_model` crate's own
//! type, which callers import from that crate), and the administrator upload's
//! [`CatalogUploadReport`].
//! **Position:** the mortar calculator reads the list and the catalog document; the catalog
//! administration page reads the upload report. The catalog document is decoded by the
//! `ballistics_model` type, so the calculator solves with exactly the values the API validated.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** every object refuses unknown fields. A catalog version is immutable once
//! stored, so a summary's `catalog_sha256` names the same catalog bytes for its whole life.
//! @contract ballistics-catalog.schema.json#/definitions/BallisticsCatalogSummary
//! @contract ballistics-catalog.schema.json#/definitions/BallisticsCatalogList
//! @contract ballistics-catalog.schema.json#/definitions/CatalogUploadReport
//! @contract ballistics-catalog.schema.json#/definitions/CalibrationFailure

use serde::{Deserialize, Serialize};

use super::identifiers::{BallisticsCatalogId, BallisticsExportGenerationId, CalibrationCaseId};

/// One stored catalog version without its weapons and shells.
/// @contract ballistics-catalog.schema.json#/definitions/BallisticsCatalogSummary
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BallisticsCatalogSummary {
    /// Lowercase slug naming the catalog across its versions.
    pub catalog_id: BallisticsCatalogId,
    /// Version number, one or more.
    pub catalog_version: u32,
    /// Human-readable catalog title.
    pub title: String,
    /// Dotted Arma Reforger build the catalog was exported from.
    pub game_build: String,
    /// Enfusion export generation GUID.
    pub export_generation_id: BallisticsExportGenerationId,
    /// Lowercase hexadecimal SHA-256 of the stored catalog bytes.
    pub catalog_sha256: String,
    /// RFC 3339 upload time, carried as sent.
    pub uploaded_at: String,
}

/// `GET /api/v1/ballistics-catalogs` answer: every stored catalog version.
/// @contract ballistics-catalog.schema.json#/definitions/BallisticsCatalogList
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BallisticsCatalogList {
    /// Every stored catalog version.
    pub data: Vec<BallisticsCatalogSummary>,
}

/// `POST /api/v1/ballistics-catalogs` outcome of flying the catalog against its calibration
/// bundle: accepted with no failures (201), or refused with every failed case (422).
/// @contract ballistics-catalog.schema.json#/definitions/CatalogUploadReport
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogUploadReport {
    /// True exactly when the calibration lists no failure.
    pub accepted: bool,
    /// Calibration cases evaluated.
    pub cases: u32,
    /// Every calibration case or provenance check the uploaded catalog failed; empty when it
    /// passed.
    pub failures: Vec<CalibrationFailure>,
    /// Forward-angle samples between native rows: the game's interpolation of its own table,
    /// neither judged nor counted in `cases`.
    pub forward_samples_not_judged: u32,
}

/// One calibration case outside tolerance, or one provenance mismatch.
/// @contract ballistics-catalog.schema.json#/definitions/CalibrationFailure
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationFailure {
    /// The case or provenance check it names.
    pub case_id: CalibrationCaseId,
    /// What differs.
    pub reason: String,
}
