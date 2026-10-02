//! The stored ballistics catalog version, as the catalog list reads it.
//!
//! **Role:** the row metadata of `ballistics_catalogs` without its catalog, calibration and
//! validation documents, and the public list that carries it.
//!
//! **Position:** operations models; written by the administrator catalog upload, read by the
//! public catalog list and by the fire-mission save that pins a catalog version.
//!
//! **Signals & state:** none; plain data.
//!
//! **Invariants:** a catalog version is immutable once stored — the table's trigger refuses every
//! update and delete — so `(catalog_id, catalog_version)` names the same bytes for its whole life,
//! and `catalog_sha256` is the SHA-256 of exactly those uploaded bytes.
//!
//! @contract ballistics-catalog.schema.json#/definitions/BallisticsCatalogSummary
//! @contract ballistics-catalog.schema.json#/definitions/BallisticsCatalogList

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::wire_format::rfc3339_utc;

/// One stored catalog version without its weapons and shells: a row of `ballistics_catalogs`
/// minus the three documents and the uploader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct BallisticsCatalogSummary {
    /// Lowercase slug naming the catalog across its versions.
    pub catalog_id: String,
    /// Version number, 1 or more; with `catalog_id` the primary key.
    pub catalog_version: i32,
    pub title: String,
    /// Arma Reforger build the catalog was exported from, dotted.
    pub game_build: String,
    /// Enfusion export generation GUID the catalog and its calibration bundle share.
    pub export_generation_id: String,
    /// Lowercase hexadecimal SHA-256 of the uploaded catalog bytes.
    pub catalog_sha256: String,
    #[serde(with = "rfc3339_utc")]
    pub uploaded_at: DateTime<Utc>,
}

/// `GET /api/v1/ballistics-catalogs` answer: every stored catalog version as a
/// [`BallisticsCatalogSummary`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BallisticsCatalogList {
    pub data: Vec<BallisticsCatalogSummary>,
}
