//! Registry model — Rust port of `internal/models/registry.go`.
//!
//! @contract registry-items.schema.json#/$defs/item

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::serde_helpers::go_time;

/// One placeable/equipable engine item in a modpack's flat catalog. Unique per
/// `(modpack_id, resource_name)`; `kind` ∈ {character, gear_primary, gear_uniform,
/// gear_vest, gear_helmet}.
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
