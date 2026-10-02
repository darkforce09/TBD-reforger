//! The identifiers of the elements on a building blueprint's level.
//!
//! **Role:** declares [`WallId`], [`DoorId`], [`WindowId`], [`StairsId`] and [`FurnitureId`],
//! each with its archived form's zero-copy reads; a door or window names the wall it sits in by
//! its [`WallId`].
//! **Position:** held by the level records of [`crate::archives::blueprints`]; the developer
//! tools' blueprint archive writer builds them and the map engine's building architecture and
//! interior line-of-sight code read them.
//! **Signals & state:** none; plain data types.
//! **Invariants:** each identifier archives exactly as the string it wraps, so the archive bytes
//! equal those of a bare `String` field; an identifier is unique within its level.

use newtype_ids::string_id;

use crate::ids::archived_accessors::archived_string_id_accessors;

string_id! {
    /// The identifier of one wall on a building level; doors and windows name their wall by it.
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    #[rkyv(derive(Debug, PartialEq, Eq, Hash))]
    pub struct WallId;
}

string_id! {
    /// The identifier of one door on a building level.
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    #[rkyv(derive(Debug, PartialEq, Eq, Hash))]
    pub struct DoorId;
}

string_id! {
    /// The identifier of one window on a building level.
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    #[rkyv(derive(Debug, PartialEq, Eq, Hash))]
    pub struct WindowId;
}

string_id! {
    /// The identifier of one staircase on a building level.
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    #[rkyv(derive(Debug, PartialEq, Eq, Hash))]
    pub struct StairsId;
}

string_id! {
    /// The identifier of one piece of furniture on a building level.
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    #[rkyv(derive(Debug, PartialEq, Eq, Hash))]
    pub struct FurnitureId;
}

archived_string_id_accessors!(WallId, ArchivedWallId);
archived_string_id_accessors!(DoorId, ArchivedDoorId);
archived_string_id_accessors!(WindowId, ArchivedWindowId);
archived_string_id_accessors!(StairsId, ArchivedStairsId);
archived_string_id_accessors!(FurnitureId, ArchivedFurnitureId);
