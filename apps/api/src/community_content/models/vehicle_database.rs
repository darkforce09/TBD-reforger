//! The vehicle database row: one vehicle of the identification table on the doctrine pages.
//!
//! **Role:** the one shape of a vehicle row, read from `vehicle_databases` and answered by the
//! vehicle database routes, and [`VehicleDatabaseList`], the list envelope that carries the rows.
//! **Position:** filled by the queries in
//! [`crate::community_content::handlers::vehicle_database`]; serialised into the list envelope
//! and the single-row answers the doctrine vehicle pages read.
//! **Signals & state:** none; plain data.
//! **Invariants:** `amphibious`, `primary_threat` and `profile_image_url` are omitted on the wire
//! when empty (the queries `COALESCE` a stored null to `''`); the lifecycle columns
//! (`created_*`, `updated_*`, `deleted_*`) never reach the wire.
//! @contract vehicle-database.schema.json#/definitions/Vehicle

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One vehicle row of the identification table.
/// @contract vehicle-database.schema.json#/definitions/Vehicle
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VehicleDatabase {
    pub id: Uuid,
    pub name: String,
    pub faction: String,
    pub armor_type: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub amphibious: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub primary_threat: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub profile_image_url: String,
}

/// The list answer: every live row, ordered by name, then id.
/// @contract vehicle-database.schema.json#/definitions/VehicleList
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleDatabaseList {
    pub data: Vec<VehicleDatabase>,
}
