//! Registry model — Rust port of `internal/models/registry.go`.
//!
//! @contract registry-items.schema.json#/$defs/item

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::serde_helpers::go_time;

/// One placeable/equipable engine item in a modpack's flat catalog. Unique per
/// `(modpack_id, resource_name)`; `kind` holds the registry-items schema kind
/// vocabulary (v3, T-068.10.2) as plain text — new kinds need no model/DDL change.
/// v3 metadata columns are nullable: v2 envelopes leave them NULL.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RegistryItem {
    pub id: Uuid,
    pub modpack_id: Uuid,
    pub resource_name: String,
    pub display_name: String,
    pub category: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub icon_url: String,
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
    /// Addon ID this prefab was scanned from (joins the envelope addons[] scan set).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub addon: Option<String>,
    pub sort_order: i64,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
    /// Feeds the weak ETag (max updated_at).
    #[serde(with = "go_time")]
    pub updated_at: DateTime<Utc>,
}

/// One directed compatibility edge: `from_node` is the item that goes in/on,
/// `to_node` the host that accepts it. Unique per `(modpack_id, from_node,
/// to_node, edge_type)`; `edge_type` holds the registry-compat schema edge
/// vocabulary as plain text — new edge families need no model/DDL change.
///
/// @contract registry-compat.schema.json#/$defs/edge
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RegistryCompatEdge {
    pub id: Uuid,
    pub modpack_id: Uuid,
    pub from_node: String,
    pub to_node: String,
    pub edge_type: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub evidence: String,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
    /// Feeds the weak ETag (max updated_at).
    #[serde(with = "go_time")]
    pub updated_at: DateTime<Utc>,
}
