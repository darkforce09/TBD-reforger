//! The prefab catalogue archive, `objects/prefabs.rkyv`, and the type inventory census.
//!
//! **Role:** declares [`PrefabCatalogArchive`] (one [`PrefabEntry`] per prefab plus the census)
//! and [`TypeInventory`], the per-kind census also shipped standalone as
//! `objects/type-inventory.rkyv`.
//! **Position:** written by the developer tools' world export catalogue writer; read by the map
//! engine's building layer and chunk loaders through
//! [`crate::archives::codec::access_checked`].
//! **Signals & state:** none; plain rkyv data types.
//! **Invariants:** the catalogue carries `schema_version`, written as
//! [`crate::archives::version::ARCHIVE_SCHEMA_VERSION`]; the type inventory carries none.

use crate::ids::{PrefabId, TerrainId};

/// Prefab entry.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct PrefabEntry {
    /// Prefab id.
    pub prefab_id: PrefabId,

    /// Kind.
    pub kind: String,

    /// Class.
    pub class: String,

    /// Class code.
    pub class_code: u8,

    /// Label.
    pub label: String,

    /// Resource name.
    pub resource_name: String,

    /// Half extents.
    pub half_extents: [f32; 3],

    /// Height m.
    pub height_m: f32,

    /// Icon key.
    pub icon_key: String,

    /// Base size px.
    pub base_size_px: f32,

    /// Default color.
    pub default_color: [u8; 4],

    /// Importance zoom.
    pub importance_zoom: f32,
}

/// Kind census.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct KindCensus {
    /// Kind.
    pub kind: String,

    /// Prefab types.
    pub prefab_types: u32,

    /// Instances.
    pub instances: u64,
}

/// Type inventory.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct TypeInventory {
    /// Terrain id.
    pub terrain_id: TerrainId,

    /// Census status.
    pub census_status: String,

    /// Unique prefabs.
    pub unique_prefabs: u32,

    /// Total instances.
    pub total_instances: u64,

    /// By kind.
    pub by_kind: Vec<KindCensus>,
}

/// Prefab catalog archive.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct PrefabCatalogArchive {
    /// Schema version.
    pub schema_version: u16,

    /// Prefabs.
    pub prefabs: Vec<PrefabEntry>,

    /// Type inventory.
    pub type_inventory: TypeInventory,
}
