//! The statements of the ballistics catalog table: the duplicate check, the audited insert and
//! the public reads.
//!
//! **Role:** every SQL statement the catalog routes run against `ballistics_catalogs`, the audit
//! line of a stored version, and the decode of a stored catalog for the fire-mission save.
//!
//! **Position:** operations services; called by
//! [`crate::operations::handlers::ballistics_catalogs`] and by any route that pins a catalog
//! version. The audit line goes through
//! [`crate::administration::services::required_audit::append_actor_audit`].
//!
//! **Signals & state:** none; each function borrows the caller's pool.
//!
//! **Invariants:**
//! - A version is written once: the insert and its audit line share one transaction, and the
//!   table's trigger refuses every later update or delete.
//! - The catalog and calibration documents are bound as the uploaded text and cast to `jsonb`
//!   by Postgres, so every number keeps the decimal digits it was uploaded with.
//! - `(catalog_id, catalog_version)` and `(catalog_id, catalog_sha256)` are each unique; either
//!   collision is a [`CatalogDuplicate`], whether the pre-check or the insert finds it.
//! - The list orders by `catalog_id ASC, catalog_version ASC`, a total order.

use sqlx::PgPool;
use website_map_engine::data::scenario::ballistics::catalog::{
    BallisticsCatalog, CatalogDecodeError,
};

use super::upload_validation::{CatalogUploadReport, DecodedUpload};
use crate::administration::services::required_audit::append_actor_audit;
use crate::core::database::postgres_errors::is_unique_violation;
use crate::operations::models::ballistics_catalog::BallisticsCatalogSummary;

/// The audit action of a stored catalog version.
pub const CATALOG_UPLOADED_AUDIT_ACTION: &str = "ballistics_catalog.uploaded";
/// The `target_type` of every catalog audit line.
pub const CATALOG_AUDIT_TARGET_TYPE: &str = "ballistics_catalog";
/// The primary-key constraint Postgres names for `(catalog_id, catalog_version)`.
const VERSION_KEY_CONSTRAINT: &str = "ballistics_catalogs_pkey";

/// The summary columns of a stored version, in [`BallisticsCatalogSummary`] field order, as a
/// literal the statements `concat!` so every statement stays a static string.
macro_rules! summary_columns {
    () => {
        "catalog_id, catalog_version, title, game_build, export_generation_id, catalog_sha256, \
         uploaded_at"
    };
}

/// Which uniqueness an upload collides with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogDuplicate {
    /// The catalog already has this version.
    VersionTaken,
    /// The catalog already has a version with these exact bytes.
    BytesTaken,
}

impl CatalogDuplicate {
    /// The stable `details.code` of the collision.
    pub fn code(self) -> &'static str {
        match self {
            Self::VersionTaken => "catalog_version_exists",
            Self::BytesTaken => "catalog_bytes_exist",
        }
    }
}

/// Why a version was not stored.
#[derive(Debug)]
pub enum StoreRefusal {
    /// Another version holds the same key.
    Duplicate(CatalogDuplicate),
    /// The database failed.
    Database(sqlx::Error),
}

impl From<sqlx::Error> for StoreRefusal {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// One accepted upload ready to store.
#[derive(Debug, Clone, Copy)]
pub struct NewCatalogVersion<'a> {
    /// The decoded pair and its digests.
    pub upload: &'a DecodedUpload,
    /// The uploaded catalog text.
    pub catalog_json: &'a str,
    /// The uploaded calibration text.
    pub calibration_json: &'a str,
    /// The report the pair was accepted by.
    pub report: &'a CatalogUploadReport,
    /// The uploading administrator's Discord id.
    pub uploaded_by: &'a str,
}

/// A stored catalog document and the SHA-256 of its uploaded bytes.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct StoredCatalogDocument {
    /// The catalog document as JSON text.
    pub catalog_document: String,
    /// Lowercase hexadecimal SHA-256 of the uploaded catalog bytes.
    pub catalog_sha256: String,
}

/// Which uniqueness, if any, a new version of `catalog_id` would collide with.
pub async fn find_duplicate(
    pool: &PgPool,
    catalog_id: &str,
    catalog_version: i32,
    catalog_sha256: &str,
) -> sqlx::Result<Option<CatalogDuplicate>> {
    let (version_taken, bytes_taken): (bool, bool) = sqlx::query_as(
        "SELECT \
           EXISTS (SELECT 1 FROM ballistics_catalogs \
                   WHERE catalog_id = $1 AND catalog_version = $2), \
           EXISTS (SELECT 1 FROM ballistics_catalogs \
                   WHERE catalog_id = $1 AND catalog_sha256 = $3)",
    )
    .bind(catalog_id)
    .bind(catalog_version)
    .bind(catalog_sha256)
    .fetch_one(pool)
    .await?;
    Ok(if version_taken {
        Some(CatalogDuplicate::VersionTaken)
    } else if bytes_taken {
        Some(CatalogDuplicate::BytesTaken)
    } else {
        None
    })
}

/// Stores one accepted version and its audit line in one transaction.
///
/// # Errors
///
/// [`StoreRefusal::Duplicate`] when either uniqueness is taken; [`StoreRefusal::Database`] for
/// any other failure, including a missing audit actor.
pub async fn store_catalog_version(
    pool: &PgPool,
    new: NewCatalogVersion<'_>,
) -> Result<BallisticsCatalogSummary, StoreRefusal> {
    let catalog = new.upload.catalog();
    let report = serde_json::to_value(new.report)
        .map_err(|error| StoreRefusal::Database(sqlx::Error::Encode(Box::new(error))))?;
    let mut transaction = pool.begin().await?;
    let inserted = sqlx::query_as::<_, BallisticsCatalogSummary>(concat!(
        "INSERT INTO ballistics_catalogs (catalog_id, catalog_version, title, game_build, \
           export_generation_id, catalog_sha256, calibration_sha256, catalog_document, \
           calibration_document, validation_report, uploaded_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9::jsonb, $10, $11) \
         RETURNING ",
        summary_columns!()
    ))
    .bind(&catalog.catalog_id)
    .bind(new.upload.catalog_version)
    .bind(&catalog.title)
    .bind(&catalog.game_build)
    .bind(&catalog.export_generation_id)
    .bind(&new.upload.pinned.sha256)
    .bind(&new.upload.calibration_sha256)
    .bind(new.catalog_json)
    .bind(new.calibration_json)
    .bind(report)
    .bind(new.uploaded_by)
    .fetch_one(&mut *transaction)
    .await;
    let summary = match inserted {
        Ok(summary) => summary,
        Err(error) if is_unique_violation(&error) => {
            let constraint = error
                .as_database_error()
                .and_then(|database_error| database_error.constraint());
            return Err(StoreRefusal::Duplicate(
                if constraint == Some(VERSION_KEY_CONSTRAINT) {
                    CatalogDuplicate::VersionTaken
                } else {
                    CatalogDuplicate::BytesTaken
                },
            ));
        }
        Err(error) => return Err(error.into()),
    };
    let message = format!(
        "uploaded ballistics catalog {} version {} ({} calibration cases, sha256 {})",
        summary.catalog_id,
        summary.catalog_version,
        new.report.calibration.cases,
        summary.catalog_sha256
    );
    append_actor_audit(
        &mut transaction,
        new.uploaded_by,
        CATALOG_UPLOADED_AUDIT_ACTION,
        CATALOG_AUDIT_TARGET_TYPE,
        &catalog_audit_target(&summary.catalog_id, summary.catalog_version),
        &message,
    )
    .await?;
    transaction.commit().await?;
    Ok(summary)
}

/// The audit `target_id` of one catalog version: `<catalog_id>/<catalog_version>`.
pub fn catalog_audit_target(catalog_id: &str, catalog_version: i32) -> String {
    format!("{catalog_id}/{catalog_version}")
}

/// Every stored version's summary, ordered by catalog then version.
pub async fn list_catalog_summaries(pool: &PgPool) -> sqlx::Result<Vec<BallisticsCatalogSummary>> {
    sqlx::query_as::<_, BallisticsCatalogSummary>(concat!(
        "SELECT ",
        summary_columns!(),
        " FROM ballistics_catalogs ORDER BY catalog_id ASC, catalog_version ASC"
    ))
    .fetch_all(pool)
    .await
}

/// One stored version's catalog document, or `None` when the version does not exist.
pub async fn find_catalog_document(
    pool: &PgPool,
    catalog_id: &str,
    catalog_version: i32,
) -> sqlx::Result<Option<StoredCatalogDocument>> {
    sqlx::query_as::<_, StoredCatalogDocument>(
        "SELECT catalog_document::text AS catalog_document, catalog_sha256 \
         FROM ballistics_catalogs WHERE catalog_id = $1 AND catalog_version = $2",
    )
    .bind(catalog_id)
    .bind(catalog_version)
    .fetch_optional(pool)
    .await
}

/// Why a stored catalog version cannot be read as a catalog.
#[derive(Debug)]
pub enum CatalogLoadError {
    /// The database failed.
    Database(sqlx::Error),
    /// The stored document no longer decodes with this build's catalog reader.
    Undecodable(CatalogDecodeError),
}

/// One stored version decoded, or `None` when the version does not exist: the catalog a
/// fire-mission save pins.
///
/// # Errors
///
/// [`CatalogLoadError::Database`] for a failed read; [`CatalogLoadError::Undecodable`] for a
/// stored document this build cannot decode.
pub async fn load_catalog(
    pool: &PgPool,
    catalog_id: &str,
    catalog_version: i32,
) -> Result<Option<BallisticsCatalog>, CatalogLoadError> {
    let Some(stored) = find_catalog_document(pool, catalog_id, catalog_version)
        .await
        .map_err(CatalogLoadError::Database)?
    else {
        return Ok(None);
    };
    BallisticsCatalog::from_json_slice(stored.catalog_document.as_bytes())
        .map(Some)
        .map_err(CatalogLoadError::Undecodable)
}
