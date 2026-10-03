//! The judgement of one uploaded catalog pair: decode, contract field checks, calibration.
//!
//! **Role:** turns the bytes of the `catalog` and `calibration` parts of a catalog upload into a
//! [`DecodedUpload`] or an [`UploadRefusal`], and runs `ballistics_calibration` over a
//! decoded pair into the [`CatalogUploadReport`] the upload answers with.
//!
//! **Position:** operations services; called by
//! [`crate::operations::handlers::ballistics_catalogs::upload`] after it has read the multipart
//! parts, before [`super::catalog_store`] stores the version. The flight model and the
//! calibration judges are [`ballistics_calibration`].
//!
//! **Signals & state:** none; pure functions over bytes and decoded documents.
//!
//! **Invariants:**
//! - A catalog decodes only through `BallisticsCatalog::from_json_slice` and a bundle only
//!   through `CalibrationBundle::from_json_slice`, so unknown fields and other schema versions
//!   are refused exactly as every other reader refuses them.
//! - A decoded catalog's identity fields hold the patterns of `ballistics-catalog.schema.json`
//!   and the `ballistics_catalogs` table checks before anything is judged or stored, so a stored
//!   row never trips a column check.
//! - `catalog_sha256` and `calibration_sha256` are the SHA-256 of the exact uploaded bytes.
//! - A report accepts its catalog exactly when the calibration lists no failure; the report
//!   carries every field of the calibration report unchanged.
//!
//! @contract ballistics-catalog.schema.json#/definitions/CatalogUploadReport

use ballistics_calibration::{CalibrationBundle, CalibrationReport, PinnedCatalog, evaluate};
use ballistics_model::catalog::BallisticsCatalog;
use serde::Serialize;

use crate::core::wire_format::content_digest::sha256_hex;

/// The longest catalog title the contract admits, in characters.
pub const MAX_TITLE_CHARS: usize = 200;
/// The longest catalog slug the contract admits, in characters.
pub const MAX_SLUG_CHARS: usize = 64;

/// `POST /api/v1/ballistics-catalogs` answer: whether the catalog is accepted and the whole
/// calibration report it was judged by.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CatalogUploadReport {
    /// True exactly when the calibration lists no failure.
    pub accepted: bool,
    /// Cases judged, every failure and the forward samples not judged.
    #[serde(flatten)]
    pub calibration: CalibrationReport,
}

impl CatalogUploadReport {
    /// The upload report of one calibration run.
    pub fn from_calibration(calibration: CalibrationReport) -> Self {
        Self {
            accepted: calibration.accepted(),
            calibration,
        }
    }
}

/// A catalog pair that decodes and whose identity fields hold the contract.
#[derive(Debug, Clone)]
pub struct DecodedUpload {
    /// The catalog with the SHA-256 of its uploaded bytes.
    pub pinned: PinnedCatalog,
    /// The catalog's version as the table's `integer` column holds it.
    pub catalog_version: i32,
    /// The calibration bundle.
    pub bundle: CalibrationBundle,
    /// Lowercase hexadecimal SHA-256 of the uploaded calibration bytes.
    pub calibration_sha256: String,
}

impl DecodedUpload {
    /// The decoded catalog.
    pub fn catalog(&self) -> &BallisticsCatalog {
        &self.pinned.catalog
    }
}

/// Why an uploaded pair is not a readable catalog and calibration bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadRefusal {
    /// The `catalog` part does not decode as a catalog document.
    CatalogUndecodable(String),
    /// The `calibration` part does not decode as a calibration bundle.
    CalibrationUndecodable(String),
    /// A decoded catalog's identity field breaks the contract.
    InvalidCatalogField {
        /// The catalog field.
        field: &'static str,
        /// What the contract requires.
        reason: String,
    },
}

impl UploadRefusal {
    /// The stable `details.code` of the refusal.
    pub fn code(&self) -> &'static str {
        match self {
            Self::CatalogUndecodable(_) => "catalog_undecodable",
            Self::CalibrationUndecodable(_) => "calibration_undecodable",
            Self::InvalidCatalogField { .. } => "invalid_catalog_field",
        }
    }

    /// The human-readable message of the refusal.
    pub fn message(&self) -> String {
        match self {
            Self::CatalogUndecodable(reason) => {
                format!("the catalog part does not decode: {reason}")
            }
            Self::CalibrationUndecodable(reason) => {
                format!("the calibration part does not decode: {reason}")
            }
            Self::InvalidCatalogField { field, reason } => {
                format!("the catalog's {field} is invalid: {reason}")
            }
        }
    }
}

/// Decodes both parts and checks the catalog's identity fields.
///
/// # Errors
///
/// [`UploadRefusal::CatalogUndecodable`] or [`UploadRefusal::CalibrationUndecodable`] for bytes
/// that are not the document; [`UploadRefusal::InvalidCatalogField`] for the first identity
/// field that breaks its pattern.
pub fn decode_upload(
    catalog_bytes: &[u8],
    calibration_bytes: &[u8],
) -> Result<DecodedUpload, UploadRefusal> {
    let pinned = PinnedCatalog::from_json_slice(catalog_bytes)
        .map_err(|error| UploadRefusal::CatalogUndecodable(error.to_string()))?;
    let catalog_version = check_catalog_identity(&pinned.catalog)?;
    let bundle = CalibrationBundle::from_json_slice(calibration_bytes)
        .map_err(|error| UploadRefusal::CalibrationUndecodable(error.to_string()))?;
    Ok(DecodedUpload {
        pinned,
        catalog_version,
        bundle,
        calibration_sha256: sha256_hex(calibration_bytes),
    })
}

/// Runs the calibration of `upload` into its upload report. CPU-bound: the caller runs it off the
/// async executor.
pub fn judge_upload(upload: &DecodedUpload) -> CatalogUploadReport {
    CatalogUploadReport::from_calibration(evaluate(&upload.pinned, &upload.bundle))
}

/// Checks the identity fields the contract patterns and the table checks constrain; answers the
/// catalog version as the table's `integer`.
fn check_catalog_identity(catalog: &BallisticsCatalog) -> Result<i32, UploadRefusal> {
    let invalid = |field: &'static str, reason: &str| UploadRefusal::InvalidCatalogField {
        field,
        reason: reason.to_owned(),
    };
    if !is_slug(catalog.catalog_id.as_str()) {
        return Err(invalid(
            "catalog_id",
            "a lowercase slug of at most 64 characters: letters and digits in groups joined by \
             single underscores or hyphens",
        ));
    }
    let catalog_version = i32::try_from(catalog.catalog_version)
        .ok()
        .filter(|version| *version >= 1)
        .ok_or_else(|| invalid("catalog_version", "an integer from 1 to 2147483647"))?;
    let title_chars = catalog.title.chars().count();
    if title_chars == 0 || title_chars > MAX_TITLE_CHARS {
        return Err(invalid("title", "between 1 and 200 characters"));
    }
    if !is_game_build(&catalog.game_build) {
        return Err(invalid(
            "game_build",
            "two to four dot-separated groups of digits",
        ));
    }
    if !is_enfusion_guid(catalog.export_generation_id.as_str()) {
        return Err(invalid(
            "export_generation_id",
            "16 uppercase hexadecimal digits",
        ));
    }
    Ok(catalog_version)
}

/// `^[a-z0-9]+([_-][a-z0-9]+)*$`, at most [`MAX_SLUG_CHARS`] characters.
fn is_slug(value: &str) -> bool {
    value.len() <= MAX_SLUG_CHARS
        && value.split(['_', '-']).all(|group| {
            !group.is_empty()
                && group
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

/// `^[0-9]+(\.[0-9]+){1,3}$`.
fn is_game_build(value: &str) -> bool {
    let groups: Vec<&str> = value.split('.').collect();
    (2..=4).contains(&groups.len())
        && groups
            .iter()
            .all(|group| !group.is_empty() && group.bytes().all(|byte| byte.is_ascii_digit()))
}

/// `^[0-9A-F]{16}$`.
fn is_enfusion_guid(value: &str) -> bool {
    value.len() == 16
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
}

#[cfg(test)]
#[path = "tests/upload_validation.rs"]
mod tests;
