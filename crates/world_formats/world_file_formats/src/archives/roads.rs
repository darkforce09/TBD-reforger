//! The road network archive, `roads/road_network.rkyv`.
//!
//! **Role:** declares [`RoadNetworkArchive`]: road segments with their class, width and
//! centreline.
//! **Position:** written by the developer tools' world export road writer; read by the map
//! engine's road network through [`crate::archives::codec::access_checked`].
//! **Signals & state:** none; plain rkyv data types.
//! **Invariants:** the archive carries `schema_version`, written as
//! [`crate::archives::version::ARCHIVE_SCHEMA_VERSION`].

use crate::ids::RoadSegmentId;

/// Road segment archive.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct RoadSegmentArchive {
    /// Id.
    pub id: RoadSegmentId,

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
