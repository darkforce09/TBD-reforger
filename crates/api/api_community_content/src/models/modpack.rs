//! Modpack manifest models: the downloadable dependency set and its nested mod rows.
//!
//! @contract modpack.schema.json#/definitions/Modpack
//! @contract modpack.schema.json#/definitions/ModpackMod

use api_identifiers::{ModGuid, ModpackId, ModpackModId, WorkshopItemId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use fleet_wire_contract::rfc3339_timestamps::rfc3339_utc;

/// Downloadable dependency set.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Modpack {
    /// The modpack's key.
    pub id: ModpackId,
    /// The display name.
    pub name: String,
    /// The pack's own version label.
    pub version: String,
    /// The download size of every mod together, in bytes.
    pub total_size_bytes: i64,
    /// The Workshop collection URL; omitted on the wire when empty.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub workshop_url: String,
    /// Whether this is the one current pack members are told to run.
    pub is_current: bool,
    /// When the row was created.
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
    /// The mod row's key.
    pub id: ModpackModId,
    /// The modpack the mod belongs to.
    pub modpack_id: ModpackId,
    /// The display name.
    pub name: String,
    /// Whether the pack is unusable without it.
    pub is_key_dependency: bool,
    /// Its position in the pack's mod list, ascending.
    pub sort_order: i64,
    /// The Reforger Workshop item id (`modId`); omitted on the wire when empty.
    #[serde(skip_serializing_if = "WorkshopItemId::is_empty", default)]
    pub workshop_id: WorkshopItemId,
    /// The mod's local GUID; omitted on the wire when empty.
    #[serde(skip_serializing_if = "ModGuid::is_empty", default)]
    pub mod_guid: ModGuid,
    /// The pinned mod version; omitted on the wire when empty.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub version: String,
}
