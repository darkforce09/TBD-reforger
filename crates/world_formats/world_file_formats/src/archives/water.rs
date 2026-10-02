//! The water vectors archive, `water/water_vectors.rkyv`.
//!
//! **Role:** declares [`WaterVectorsArchive`]: lake and pond polygons and river lines.
//! **Position:** written by the developer tools' map raster pipeline; read by the map engine's
//! water layer through [`crate::archives::codec::access_checked`].
//! **Signals & state:** none; plain rkyv data types.
//! **Invariants:** the archive carries `schema_version`, written as
//! [`crate::archives::version::ARCHIVE_SCHEMA_VERSION`].

use crate::ids::WaterFeatureId;

/// Water body.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct WaterBody {
    /// Id.
    pub id: WaterFeatureId,

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
    pub id: WaterFeatureId,

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
