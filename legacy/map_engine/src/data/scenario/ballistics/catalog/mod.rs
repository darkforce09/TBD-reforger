//! Game ballistics catalog: the weapons and shells the fire-mission solver flies.
//!
//! **Role:** the typed, serde form of one catalog document, its decode from JSON, and (through
//! [`lookup`]) typed lookups and the flight model inputs of one firing.
//!
//! **Position:** `data/scenario/ballistics`; the API decodes uploaded and stored catalogs and
//! the frontend decodes the catalog it downloads, both into [`BallisticsCatalog`]; the solver,
//! calibration, dispersion and fuze modules read it through [`lookup`].
//!
//! **Signals & state:** none; plain data and pure functions.
//!
//! **Invariants:**
//! - Field names and shapes match `contracts/definitions/ballistics-catalog.schema.json`;
//!   every object refuses unknown fields.
//! - [`BallisticsCatalog::from_json_slice`] accepts only [`CATALOG_SCHEMA_VERSION`].
//! - Values are carried exactly as decoded; nothing is rounded, reordered or defaulted except an
//!   absent time fuze.
//!
//! @contract ballistics-catalog.schema.json#/definitions/BallisticsCatalog
//! @contract ballistics-catalog.schema.json#/definitions/CatalogResource

pub mod lookup;
pub mod shell;
pub mod weapon;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Re-exports the lookup refusals, the resolved firing and the derivations.
pub use lookup::{CatalogLookupError, ResolvedFiring, flight_parameters, muzzle_speed_m_s};
/// Re-exports the shell, its role, charges and time fuze.
pub use shell::{Charge, Shell, ShellRole, TimeFuze};
/// Re-exports the launcher.
pub use weapon::WeaponSystem;

/// The only catalog document version this crate reads.
pub const CATALOG_SCHEMA_VERSION: u32 = 1;

/// One immutable catalog version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BallisticsCatalog {
    /// Document version; [`CATALOG_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Lowercase slug naming the catalog.
    pub catalog_id: String,
    /// Version of the catalog, one or more.
    pub catalog_version: u32,
    /// Human-readable title.
    pub title: String,
    /// Dotted Arma Reforger build the values were exported from.
    pub game_build: String,
    /// Enfusion GUID of the equipment export generation.
    pub export_generation_id: String,
    /// Gravitational acceleration in metres per second squared.
    pub gravity_m_s2: f64,
    /// Where [`Self::gravity_m_s2`] comes from.
    pub gravity_source: GravitySource,
    /// Every game resource the values were read from.
    pub resources: Vec<CatalogResource>,
    /// Every launcher.
    pub weapons: Vec<WeaponSystem>,
    /// Every shell.
    pub shells: Vec<Shell>,
}

/// Where a catalog's gravity comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GravitySource {
    /// The engine oracle run recorded in the calibration bundle.
    Oracle,
}

/// One game resource the catalog's values were read from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogResource {
    /// Enfusion GUID: 16 uppercase hexadecimal digits.
    pub guid: String,
    /// The resource's name.
    pub resource_name: String,
    /// Lowercase hexadecimal SHA-256 of the exported bytes.
    pub sha256: String,
}

/// Why bytes are not a readable catalog document.
#[derive(Debug, Error)]
pub enum CatalogDecodeError {
    /// The bytes are not JSON of the catalog's shape.
    #[error("catalog JSON does not decode: {0}")]
    Json(#[from] serde_json::Error),
    /// The document declares a version this crate does not read.
    #[error("catalog schema_version {0} is not supported (expected {CATALOG_SCHEMA_VERSION})")]
    UnsupportedSchemaVersion(u32),
}

impl BallisticsCatalog {
    /// Decodes one catalog document.
    ///
    /// # Errors
    ///
    /// [`CatalogDecodeError::Json`] for malformed JSON, a missing or unknown field or a value of
    /// the wrong type; [`CatalogDecodeError::UnsupportedSchemaVersion`] for any version but
    /// [`CATALOG_SCHEMA_VERSION`].
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, CatalogDecodeError> {
        let catalog: Self = serde_json::from_slice(bytes)?;
        if catalog.schema_version != CATALOG_SCHEMA_VERSION {
            return Err(CatalogDecodeError::UnsupportedSchemaVersion(
                catalog.schema_version,
            ));
        }
        Ok(catalog)
    }
}

#[cfg(test)]
#[path = "tests/catalog_decode.rs"]
mod tests;
