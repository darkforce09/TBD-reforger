//! Role: water.
//! Position: `io/archives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Water body.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct WaterBody {
    /// Id.
    pub id: String,

    /// Surface y.
    pub surface_y: f32,

    /// Ring.
    pub ring: Vec<[f32; 2]>,
}

/// Water line.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct WaterLine {
    /// Id.
    pub id: String,

    /// Width m.
    pub width_m: f32,

    /// Centerline.
    pub centerline: Vec<[f32; 2]>,
}

/// Water vectors archive.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct WaterVectorsArchive {
    /// Schema version.
    pub schema_version: u16,

    /// Lakes.
    pub lakes: Vec<WaterBody>,

    /// Rivers.
    pub rivers: Vec<WaterLine>,

    /// Ponds.
    pub ponds: Vec<WaterBody>,
}
