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

use serde::{Deserialize, Serialize};

pub use website_mission_core::doc::operations::faction_library::FactionDoc;
pub use website_mission_core::doc::operations::faction_library::FactionRole;
pub use website_mission_core::doc::operations::faction_library::FactionVehicle;

/// One entry in the asset catalogue: what it is, where it lives, and how it is shown.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryItem {
    pub id: String,
    pub modpack_id: String,
    pub resource_name: String,
    pub display_name: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub kind: String,
    #[serde(rename = "abstract", default, skip_serializing_if = "Option::is_none")]
    pub r#abstract: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arsenal_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight_kg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_cm3: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_weight_kg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_volume_cm3: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_grid_w: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_grid_h: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addon: Option<String>,
    /// The base weapon this item is an attachment or camouflage variant of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant_of: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// A page of registry entries.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryResponse {
    pub data: Vec<RegistryItem>,
    pub etag: String,
    pub modpack_id: String,
    pub modpack_version: String,
    /// Present only when the request asked for a page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// One edge of the compatibility graph: what fits into what, and how many.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCompatEdge {
    pub id: String,
    pub modpack_id: String,
    pub from_node: String,
    pub to_node: String,
    pub edge_type: String,
    #[serde(default)]
    pub evidence: String,
    /// How many of the child the edge stands for. Duplicate emissions from the scanner are
    /// aggregated here; every other family carries one.
    #[serde(default = "default_edge_qty")]
    pub qty: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// The quantity an edge carries when the payload does not name one.
fn default_edge_qty() -> i64 {
    1
}

/// The compatibility edges for a set of registry entries.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCompatResponse {
    pub data: Vec<RegistryCompatEdge>,
    pub etag: String,
    pub modpack_id: String,
    pub modpack_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// The default cargo one character prefab is issued.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCargoDefaultRow {
    pub container: String,
    pub item: String,
    pub qty: i64,
}

/// Default cargo for a set of character prefabs.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCargoDefaultsResponse {
    /// Echo of the requested view.
    pub view: String,
    pub data: std::collections::HashMap<String, Vec<RegistryCargoDefaultRow>>,
    pub etag: String,
    pub modpack_id: String,
    pub modpack_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_edge_count: Option<i64>,
}

/// A faction as it appears against one user.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct UserFaction {
    pub id: String,
    pub owner_id: String,
    pub side: String,
    pub name: String,
    pub doc: FactionDoc,
    pub created_at: String,
    pub updated_at: String,
}

/// Every faction available to the viewer.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionListResponse {
    pub data: Vec<UserFaction>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// The four sides a faction can belong to, in the order the pickers show them.
pub const FACTION_SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR", "CIV"];
