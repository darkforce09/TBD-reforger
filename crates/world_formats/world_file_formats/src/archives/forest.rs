//! The forest regions archive, `objects/forest-regions.rkyv`.
//!
//! **Role:** declares [`ForestRegionsArchive`]: land-cover polygons with their tree counts and
//! density.
//! **Position:** written by the developer tools' world export catalogue writer; read by the map
//! engine's vegetation layer through [`crate::archives::codec::access_checked`].
//! **Signals & state:** none; plain rkyv data types.
//! **Invariants:** the archive carries `schema_version`, written as
//! [`crate::archives::version::ARCHIVE_SCHEMA_VERSION`].

use crate::ids::ForestRegionId;

/// Forest region.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct ForestRegion {
    /// Id.
    pub id: ForestRegionId,

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
