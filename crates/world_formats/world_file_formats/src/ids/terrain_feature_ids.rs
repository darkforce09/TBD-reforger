//! The identifiers of a terrain and of the features its archives describe.
//!
//! **Role:** declares [`TerrainId`] (the terrain a type inventory census counts),
//! [`RoadSegmentId`] (one segment of the road network), [`ForestRegionId`] (one land-cover
//! polygon) and [`WaterFeatureId`] (one lake, pond or river of the water vectors), each with its
//! archived form's zero-copy reads.
//! **Position:** held by the records of [`crate::archives::prefabs`], [`crate::archives::roads`],
//! [`crate::archives::forest`] and [`crate::archives::water`]; the developer tools build them from
//! the exported JSON and the map engine reads them from the archives.
//! **Signals & state:** none; plain data types.
//! **Invariants:** each identifier archives exactly as the string it wraps, so the archive bytes
//! equal those of a bare `String` field.

use newtype_ids::string_id;

use crate::ids::archived_accessors::archived_string_id_accessors;

string_id! {
    /// A terrain's identifier, such as `everon`, as the type inventory census records it.
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    #[rkyv(derive(Debug, PartialEq, Eq, Hash))]
    pub struct TerrainId;
}

string_id! {
    /// The identifier of one segment of a terrain's road network.
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    #[rkyv(derive(Debug, PartialEq, Eq, Hash))]
    pub struct RoadSegmentId;
}

string_id! {
    /// The identifier of one forest or land-cover region polygon.
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    #[rkyv(derive(Debug, PartialEq, Eq, Hash))]
    pub struct ForestRegionId;
}

string_id! {
    /// The identifier of one water feature: a lake, a pond or a river line.
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    #[rkyv(derive(Debug, PartialEq, Eq, Hash))]
    pub struct WaterFeatureId;
}

archived_string_id_accessors!(TerrainId, ArchivedTerrainId);
archived_string_id_accessors!(RoadSegmentId, ArchivedRoadSegmentId);
archived_string_id_accessors!(ForestRegionId, ArchivedForestRegionId);
archived_string_id_accessors!(WaterFeatureId, ArchivedWaterFeatureId);
