//! Role: prefabs.
//! Position: `formats/archives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Prefab entry.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct PrefabEntry {
    /// Prefab id.
    pub prefab_id: u32,

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
    pub terrain_id: String,

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
