//! Role: labels.
//! Position: `io/archives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

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
