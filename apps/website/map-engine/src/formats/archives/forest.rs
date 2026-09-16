//! Role: forest.
//! Position: `formats/archives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Forest region.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct ForestRegion {
    /// Id.
    pub id: String,

    /// Kind.
    pub kind: String,

    /// Polygon.
    pub polygon: Vec<Vec<[f32; 2]>>,

    /// Tree count.
    pub tree_count: u32,

    /// Dominant species class.
    pub dominant_species_class: String,

    /// Density per ha.
    pub density_per_ha: f32,

    /// Area ha.
    pub area_ha: f32,

    /// Cover type.
    pub cover_type: String,
}

/// Forest regions archive.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct ForestRegionsArchive {
    /// Schema version.
    pub schema_version: u16,

    /// Regions.
    pub regions: Vec<ForestRegion>,
}
