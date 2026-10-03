//! The identifiers a building's interior model names its prefab, its placed instances and the
//! features a sight line meets by.
//!
//! **Role:** declares [`BuildingPrefabId`] (a building prefab's slug, such as
//! `FarmHouse_E_1L01`), [`CompoundInstanceId`] (one placed instance of a compound, the prefab
//! child's `ID` chain joined by `/`) and [`BuildingFeatureId`] (the blueprint feature a sight-line
//! event names: a wall, window, door, stairs, furniture or compound instance id, or `roof` or
//! `solid`).
//! **Position:** held by [`crate::blueprint::structure::BuildingBlueprint`],
//! [`crate::compound::instances`] and [`crate::blueprint::sight_line::LosHit`]; the wall, door,
//! window, stairs and furniture identifiers themselves are `world_file_formats::ids`; the map
//! engine's compound walk names an instance's feature by its compound instance id.
//! **Signals & state:** none; plain data types declared with the `newtype_ids` macros.
//! **Invariants:** each identifier serializes exactly as the string it wraps, so the blueprint and
//! instances JSON documents read and write the same bytes as with a bare `String` field.

use newtype_ids::string_id;

string_id! {
    /// A building prefab's slug, such as `FarmHouse_E_1L01`, naming its blueprint and instances
    /// files.
    pub struct BuildingPrefabId;
}

string_id! {
    /// One placed instance of a compound: the prefab child's `ID` chain joined by `/`, stable
    /// within the building.
    pub struct CompoundInstanceId;
}

string_id! {
    /// The feature a sight-line event names: a blueprint wall, window, door, stairs or furniture
    /// identifier, a compound instance identifier, or `roof` or `solid` for a hit no other
    /// feature claims.
    pub struct BuildingFeatureId;
}
