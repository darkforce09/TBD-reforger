//! Registry model — Rust port of `internal/models/registry.go`.
//!
//! @contract registry-items.schema.json#/$defs/item

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::serde_helpers::go_time;

/// One placeable/equipable engine item in a modpack's flat catalog. Unique per
/// `(modpack_id, resource_name)`; `kind` holds the registry-items schema kind
/// vocabulary (v2, T-150) as plain text — new kinds need no model/DDL change.
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
