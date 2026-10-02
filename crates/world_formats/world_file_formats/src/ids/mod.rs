//! The identifier types the archive records and the object row carry.
//!
//! **Role:** gives every identifier field of the world files a type of its own: the prefab
//! catalogue identifiers ([`prefab_ids`]), the terrain and its archived features
//! ([`terrain_feature_ids`]) and the elements of a building level ([`building_element_ids`]).
//! **Position:** the records under [`crate::archives`] and the row in [`crate::pod`] hold these
//! types; the developer tools construct them and the map engine reads them, natively and through
//! the archived forms' accessors.
//! **Signals & state:** none; plain data types declared with the `newtype_ids` macros.
//! **Invariants:** an identifier has its inner value's exact byte layout in rkyv, in the `Pod`
//! row and in serde, so no archive, row or JSON byte depends on whether a field is an identifier
//! type or its bare primitive.

mod archived_accessors;
pub mod building_element_ids;
pub mod prefab_ids;
pub mod terrain_feature_ids;

pub use building_element_ids::{DoorId, FurnitureId, StairsId, WallId, WindowId};
pub use prefab_ids::{InstancePrefabId, PrefabId};
pub use terrain_feature_ids::{ForestRegionId, RoadSegmentId, TerrainId, WaterFeatureId};

#[cfg(test)]
#[path = "tests/identifier_accessor_tests.rs"]
mod identifier_accessor_tests;
