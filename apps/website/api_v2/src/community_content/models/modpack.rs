//! Modpack manifest models: the downloadable dependency set and its nested mod rows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::rfc3339_utc;

/// Downloadable dependency set.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Modpack {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub total_size_bytes: i64,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub workshop_url: String,
    pub is_current: bool,
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}

/// One mod inside a modpack.
///
/// `workshop_id` / `mod_guid` / `version` map onto a Reforger `game.mods[]` entry
/// (`modId`, local GUID, optional version pin). Empty strings are omitted on the wire
/// (same pattern as [`Modpack::workshop_url`]).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ModpackMod {
    pub id: Uuid,
    pub modpack_id: Uuid,
    pub name: String,
    pub is_key_dependency: bool,
    pub sort_order: i64,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub workshop_id: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub mod_guid: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub version: String,
}
