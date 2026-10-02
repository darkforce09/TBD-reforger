//! The map labels archive, `locations/map_labels.rkyv`.
//!
//! **Role:** declares [`MapLabelsArchive`]: town, height and road name labels.
//! **Position:** written by the developer tools' map raster pipeline; read by the map engine's
//! location labels through [`crate::archives::codec::access_checked`].
//! **Signals & state:** none; plain rkyv data types.
//! **Invariants:** the archive carries `schema_version`, written as
//! [`crate::archives::version::ARCHIVE_SCHEMA_VERSION`]; road name labels are stored in
//! declutter order.

/// Town label.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct TownLabel {
    /// Name.
    pub name: String,

    /// Position.
    pub position: [f32; 2],

    /// Importance.
    pub importance: f32,

    /// Kind.
    pub kind: String,
}

/// Height label.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct HeightLabel {
    /// Position.
    pub position: [f32; 2],

    /// Elevation m.
    pub elevation_m: f32,
}

/// Road name label.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct RoadNameLabel {
    /// Name.
    pub name: String,

    /// Position.
    pub position: [f32; 2],

    /// Angle deg.
    pub angle_deg: f32,

    /// Road class.
    pub road_class: u8,
}

/// Map labels archive.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct MapLabelsArchive {
    /// Schema version.
    pub schema_version: u16,

    /// Towns.
    pub towns: Vec<TownLabel>,

    /// Height labels.
    pub height_labels: Vec<HeightLabel>,

    /// Road names.
    pub road_names: Vec<RoadNameLabel>,
}
