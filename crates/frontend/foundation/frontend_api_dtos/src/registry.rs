//! The asset registry: prefabs, their compatibility graph, and faction rosters.
//!
//! **Role:** the catalogue the editor's palette is built from, the compatibility edges that say
//! what fits into what, and the faction documents that group roles and vehicles.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** resource paths are the identity of a registry item — two items never share one. A
//! compatibility edge carries a quantity that defaults to one, so a payload written before
//! quantities existed still reads correctly.
//! @contract arsenal-envelopes.schema.json#/definitions/RegistryItemPage
//! @contract arsenal-envelopes.schema.json#/definitions/RegistryCompatPage
//! @contract arsenal-envelopes.schema.json#/definitions/RegistryCargoDefaults
//! @contract arsenal-envelopes.schema.json#/definitions/FactionList

use serde::{Deserialize, Serialize};

use super::identifiers::{
    DiscordUserId, ModpackId, RegistryCompatibilityId, RegistryItemId, UserFactionId,
};
use mission_operations::faction_library::FactionDoc;

/// One entry in the asset catalogue: what it is, where it lives, and how it is shown.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryItem {
    /// Primary key (uuid).
    pub id: RegistryItemId,
    /// The modpack whose catalog holds this item.
    pub modpack_id: ModpackId,
    /// Enfusion ResourceName of the prefab (`{GUID}Prefabs/.../File.et`).
    pub resource_name: String,
    /// Human-readable item name shown in pickers.
    pub display_name: String,
    /// Slash-delimited browse path, e.g. `NATO/Rifleman`.
    pub category: String,
    /// Icon URL; empty (omitted on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    /// Item classification from the registry-items schema kind vocabulary.
    pub kind: String,
    /// Whether the item is an abstract base that is never placed itself; absent when unknown.
    #[serde(rename = "abstract", default, skip_serializing_if = "Option::is_none")]
    pub r#abstract: Option<bool>,
    /// SCR_EArsenalItemType flag name when the item has a faction EntityCatalog entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arsenal_type: Option<String>,
    /// ItemPhysicalAttributes.Weight (kg); absent = the engine class default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight_kg: Option<f64>,
    /// ItemPhysicalAttributes.ItemVolume (cm³); absent = the engine class default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_cm3: Option<f64>,
    /// Container carry capacity (m_fMaxWeight, kg) when the item is itself a container.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_weight_kg: Option<f64>,
    /// Container volume capacity (MaxCumulativeVolume, cm³) when the item is a container.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_volume_cm3: Option<f64>,
    /// Inventory UI grid width in cells (scanner-derived, VOLUME_PER_CELL=50, w=4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_grid_w: Option<i64>,
    /// Inventory UI grid height in cells (h = max(3, ceil(cells/4))).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_grid_h: Option<i64>,
    /// Addon ID this prefab was scanned from (joins the envelope addons[] scan set).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addon: Option<String>,
    /// The base weapon this item is an attachment or camouflage variant of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant_of: Option<String>,
    /// Ascending display order within the catalog.
    pub sort_order: i64,
    /// When the row was imported (RFC 3339 UTC).
    pub created_at: String,
    /// Feeds the weak ETag (max updated_at).
    pub updated_at: String,
}

/// A page of registry entries.
/// @contract arsenal-envelopes.schema.json#/definitions/RegistryItemPage
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryResponse {
    /// The registry's items.
    pub data: Vec<RegistryItem>,
    /// The entity tag of this registry state, for conditional reads.
    pub etag: String,
    /// The modpack the registry describes.
    pub modpack_id: ModpackId,
    /// The version of that modpack.
    pub modpack_version: String,
    /// Present only when the request asked for a page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    /// The page size applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// The number of deployments skipped before this page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// One edge of the compatibility graph: what fits into what, and how many.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCompatEdge {
    /// Primary key (uuid).
    pub id: RegistryCompatibilityId,
    /// The modpack whose graph holds this edge.
    pub modpack_id: ModpackId,
    /// ResourceName of the item that goes in or on the host.
    pub from_node: String,
    /// ResourceName of the host that accepts the item.
    pub to_node: String,
    /// Edge family from the registry-compat schema vocabulary, e.g. `mag_in_weapon`.
    pub edge_type: String,
    /// Engine class or container var that proved the edge; empty (omitted on the wire) if unset.
    #[serde(default)]
    pub evidence: String,
    /// How many of the child the edge stands for. Duplicate emissions from the scanner are
    /// aggregated here; every other family carries one.
    #[serde(default = "default_edge_qty")]
    pub qty: i64,
    /// When the edge was imported (RFC 3339 UTC).
    pub created_at: String,
    /// Feeds the weak ETag (max updated_at).
    pub updated_at: String,
}

/// The quantity an edge carries when the payload does not name one.
fn default_edge_qty() -> i64 {
    1
}

/// The compatibility edges for a set of registry entries.
/// @contract arsenal-envelopes.schema.json#/definitions/RegistryCompatPage
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCompatResponse {
    /// The registry's compatibility edges.
    pub data: Vec<RegistryCompatEdge>,
    /// The entity tag of this registry state, for conditional reads.
    pub etag: String,
    /// The modpack the registry describes.
    pub modpack_id: ModpackId,
    /// The version of that modpack.
    pub modpack_version: String,
    /// Total number of the server's deployments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    /// The page size applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// The number of deployments skipped before this page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// The default cargo one character prefab is issued.
/// @contract arsenal-envelopes.schema.json#/definitions/CargoDefaultRow
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCargoDefaultRow {
    /// Worn container the item goes into: `vest`, `pants`, `jacket` or `backpack`.
    pub container: String,
    /// ResourceName of the cargo item.
    pub item: String,
    /// Number of items, at least one.
    pub qty: i64,
}

/// Default cargo for a set of character prefabs.
/// @contract arsenal-envelopes.schema.json#/definitions/RegistryCargoDefaults
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCargoDefaultsResponse {
    /// Echo of the requested view.
    pub view: String,
    /// The default cargo rows, keyed by the carrying item.
    pub data: std::collections::HashMap<String, Vec<RegistryCargoDefaultRow>>,
    /// The entity tag of this registry state, for conditional reads.
    pub etag: String,
    /// The modpack the registry describes.
    pub modpack_id: ModpackId,
    /// The version of that modpack.
    pub modpack_version: String,
    /// How many compatibility edges the defaults were derived from; absent when unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_edge_count: Option<i64>,
}

/// A faction as it appears against one user.
/// @contract arsenal-envelopes.schema.json#/definitions/UserFaction
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct UserFaction {
    /// Primary key (uuid).
    pub id: UserFactionId,
    /// Discord id of the operator who owns the faction.
    pub owner_id: DiscordUserId,
    /// Side key projected from the document: `BLUFOR`, `OPFOR`, `INDFOR` or `CIV`.
    pub side: String,
    /// Display name projected from the document; unique per owner.
    pub name: String,
    /// The full faction-library document, passed through as the API stored it.
    pub doc: FactionDoc,
    /// When the faction was created (RFC 3339 UTC).
    pub created_at: String,
    /// When the faction last changed (RFC 3339 UTC).
    pub updated_at: String,
}

/// Every faction available to the viewer.
/// @contract arsenal-envelopes.schema.json#/definitions/FactionList
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionListResponse {
    /// The caller's factions on this page.
    pub data: Vec<UserFaction>,
    /// Total number of the server's deployments.
    pub total: i64,
    /// The page size applied.
    pub limit: i64,
    /// The number of deployments skipped before this page.
    pub offset: i64,
}

/// The four sides a faction can belong to, in the order the pickers show them.
pub const FACTION_SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR", "CIV"];
