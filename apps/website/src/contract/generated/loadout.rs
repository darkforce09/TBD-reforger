// Code generated from JSON Schema using quicktype. DO NOT EDIT.
// Source: packages/tbd-schema/schema/loadout-export.schema.json — regenerate with: make schema-codegen

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::loadout;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: loadout = serde_json::from_str(&json).unwrap();
// }

use serde::{Deserialize, Serialize};

/// Loadout download consumed by the mod equip path. v1: ACE-shaped fixed gear slots. v2
/// (T-068.10.4): Reforger-shaped — wear is an open map keyed by engine LoadoutSlotInfo name
/// (canonical keys documented; mod-added areas allowed), weapons are slot-indexed (two
/// untyped primary slots + secondary + grenade/throwable per Character_Base.et),
/// equipment/cargo are forward skeletons. v2 keeps a derived legacy gear block so the v1 mod
/// reader (TBD_LoadoutEquipComponent, JsonLoadContext ignores unknown fields — U6) keeps
/// working until T-068.12 reads v2 natively.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Loadout {
    pub gear: Gear,

    pub loadout_version: LoadoutVersion,

    pub modpack_id: String,

    /// Container cargo (volume/weight budget model — no grid cells). Skeleton in v2 — UI and
    /// budget validation land with the cargo slice.
    pub cargo: Option<Vec<Cargo>>,

    /// Equipment micro-slots (SCR_EquipmentStorageComponent): binoculars, wristwatch, … Skeleton
    /// in v2 — UI lands with the equipment slice.
    pub equipment: Option<Equipment>,

    /// Slot-indexed weapons. Vanilla characters: slotIndex 0/1 slotType 'primary' (two untyped
    /// long slots — two rifles legal), 2 'secondary' (pistol), 3 'grenade', 4 'throwable'.
    /// T-068.12 must equip via slot-indexed SetWeapon, not blind EquipWeapon.
    pub weapons: Option<Vec<LoadoutExportSchema>>,

    /// Wear areas keyed by engine LoadoutSlotInfo name. Canonical keys: headCover, jacket,
    /// pants, boots, vest, armoredVest, backpack, handwear (Character_Base.et); pattern-open so
    /// mod-added LoadoutAreaType subclasses are representable without a schema change.
    pub wear: Option<Wear>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Cargo {
    pub container: String,

    pub item: String,

    pub qty: i64,
}

/// Equipment micro-slots (SCR_EquipmentStorageComponent): binoculars, wristwatch, … Skeleton
/// in v2 — UI lands with the equipment slice.
#[derive(Debug, Serialize, Deserialize)]
pub struct Equipment {}

/// v1 fixed gear slots. In v2 envelopes this block is DERIVED (jacket→uniform, armoredVest
/// else vest→vest, headCover→helmet, weapons[0]→primary/optic/magazine) for the v1 mod
/// reader; T-068.12 switches to the v2 fields.
#[derive(Debug, Serialize, Deserialize)]
pub struct Gear {
    pub helmet: Option<String>,

    pub magazine: Option<String>,

    pub optic: Option<String>,

    pub primary: Option<String>,

    pub uniform: Option<String>,

    pub vest: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LoadoutVersion {
    #[serde(rename = "1")]
    The1,

    #[serde(rename = "2")]
    The2,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadoutExportSchema {
    pub attachments: Option<Vec<String>>,

    pub magazine: Option<String>,

    pub optic: Option<String>,

    pub slot_index: i64,

    pub slot_type: String,

    pub weapon: String,
}

/// Wear areas keyed by engine LoadoutSlotInfo name. Canonical keys: headCover, jacket,
/// pants, boots, vest, armoredVest, backpack, handwear (Character_Base.et); pattern-open so
/// mod-added LoadoutAreaType subclasses are representable without a schema change.
#[derive(Debug, Serialize, Deserialize)]
pub struct Wear {}
