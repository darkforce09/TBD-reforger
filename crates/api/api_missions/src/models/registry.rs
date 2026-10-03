//! Registry models: the stored rows of the flat per-modpack item catalog and of the directed
//! compatibility graph that says what goes in or on what, as the registry pages answer them.
//!
//! @contract arsenal-envelopes.schema.json#/definitions/RegistryItemRow
//! @contract arsenal-envelopes.schema.json#/definitions/RegistryCompatRow

use api_identifiers::{ModpackId, RegistryCompatibilityId, RegistryItemId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use fleet_wire_contract::rfc3339_timestamps::rfc3339_utc;

/// Serde default for [`RegistryCompatEdge::qty`] — an edge with no stated multiplicity is one.
fn default_edge_qty() -> i32 {
    1
}

/// One placeable/equipable engine item in a modpack's flat catalog. Unique per
/// `(modpack_id, resource_name)`; `kind` holds the registry-items schema kind
/// vocabulary (v3) as plain text — new kinds need no model/DDL change.
/// v3 metadata columns are nullable: v2 envelopes leave them NULL.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub icon_url: String,
    /// Item classification from the registry-items schema kind vocabulary.
    pub kind: String,
    /// Non-placeable template prefab (`*_base.et` / `* Base`) — hidden from pickers.
    #[serde(rename = "abstract", skip_serializing_if = "Option::is_none", default)]
    #[sqlx(rename = "abstract")]
    pub abstract_: Option<bool>,
    /// SCR_EArsenalItemType flag name when the item has a faction EntityCatalog entry.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub arsenal_type: Option<String>,
    /// ItemPhysicalAttributes.Weight (kg); NULL = engine class default (not serialized).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub weight_kg: Option<f64>,
    /// ItemPhysicalAttributes.ItemVolume (cm³); NULL = engine class default.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub volume_cm3: Option<f64>,
    /// Container carry capacity (m_fMaxWeight, kg) when the item is itself a container.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_weight_kg: Option<f64>,
    /// Container volume capacity (MaxCumulativeVolume, cm³) when the item is a container.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_volume_cm3: Option<f64>,
    /// Inventory UI grid width in cells (scanner-derived, VOLUME_PER_CELL=50, w=4).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cargo_grid_w: Option<i32>,
    /// Inventory UI grid height in cells (h = max(3, ceil(cells/4))).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cargo_grid_h: Option<i32>,
    /// Addon ID this prefab was scanned from (joins the envelope addons[] scan set).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub addon: Option<String>,
    /// Base weapon this row is a factory attachment/camo configuration of.
    /// Pickers hide variant rows; NULL for base weapons and non-weapons.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub variant_of: Option<String>,
    /// Ascending display order within the catalog.
    pub sort_order: i64,
    /// When the row was imported (RFC 3339 UTC).
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    /// Feeds the weak ETag (max updated_at).
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}

/// One directed compatibility edge: `from_node` is the item that goes in/on,
/// `to_node` the host that accepts it. Unique per `(modpack_id, from_node,
/// to_node, edge_type, COALESCE(evidence, ''))`; `edge_type` holds
/// the registry-compat schema edge vocabulary as plain text — new edge families
/// need no model/DDL change.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub evidence: String,
    /// Edge multiplicity: the scanner emits `character_default_cargo`
    /// once per `PrefabsToSpawn` entry; the importer aggregates duplicates here.
    /// 1 for every other edge family.
    #[serde(default = "default_edge_qty")]
    pub qty: i32,
    /// When the edge was imported (RFC 3339 UTC).
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    /// Feeds the weak ETag (max updated_at).
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}
