//! Role: roads.
//! Position: `io/archives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Road segment archive.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct RoadSegmentArchive {
    /// Id.
    pub id: String,

    /// Road class.
    pub road_class: u8,

    /// Width m.
    pub width_m: f32,

    /// Centerline.
    pub centerline: Vec<[f32; 2]>,
}

/// Road network archive.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct RoadNetworkArchive {
    /// Schema version.
    pub schema_version: u16,

    /// Segments.
    pub segments: Vec<RoadSegmentArchive>,
}
