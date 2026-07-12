// Code generated from JSON Schema using quicktype. DO NOT EDIT.
// Source: packages/tbd-schema/schema/faction-library.schema.json — regenerate with: make schema-codegen

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::faction_library;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: faction_library = serde_json::from_str(&json).unwrap();
// }

use serde::{Deserialize, Serialize};

/// One operator-authored reusable faction (T-152): a side + display name plus its ORBAT role
/// templates (each wrapping a registry character with an optional SlotLoadout v2) and its
/// vehicle pool. Stored as the jsonb doc of a user_factions row; the Mission Creator palette
/// renders side → faction → roles/vehicles from these instead of the raw vanilla registry
/// dump. Role loadouts reuse the loadout-export v2 shapes (wear open map keyed by engine
/// LoadoutSlotInfo name; slot-indexed weapons).
#[derive(Debug, Serialize, Deserialize)]
pub struct FactionLibrary {
    /// Optional emblem asset path/URL (UI later).
    pub emblem: Option<String>,

    /// Display name, e.g. 'US Army 1980s'.
    pub name: String,

    /// ORBAT role templates in authored order — the palette's draggable leaves.
    pub roles: Vec<RoleElement>,

    /// Export-side key (mirrors the mission doc Faction.key vocabulary).
    pub side: Side,

    /// Vehicle pool (listed in the palette; map placement lands with T-070).
    pub vehicles: Vec<VehicleElement>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleElement {
    /// Registry character (kind === character) this role wraps — vanilla bodies are fine here;
    /// the palette hides them, roles don't.
    pub character: String,

    pub loadout: Option<Loadout>,

    pub role: String,

    pub tag: Option<String>,
}

/// SlotLoadout v2 (mirrors loadout-export.schema.json v2 doc shapes).
#[derive(Debug, Serialize, Deserialize)]
pub struct Loadout {
    pub cargo: Option<Vec<Cargo>>,

    pub equipment: Option<Equipment>,

    pub summary: Option<String>,

    /// SlotLoadout v2 marker (const 2 — expressed as bounds for the quicktype Rust emitter).
    pub version: i64,

    pub weapons: Vec<Weapon>,

    pub wear: Wear,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Cargo {
    pub container: String,

    pub item: String,

    pub qty: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Equipment {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Weapon {
    pub attachments: Option<Vec<String>>,

    pub magazine: Option<String>,

    pub optic: Option<String>,

    pub slot_index: i64,

    pub slot_type: String,

    pub weapon: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Wear {}

/// Export-side key (mirrors the mission doc Faction.key vocabulary).
#[derive(Debug, Serialize, Deserialize)]
pub enum Side {
    #[serde(rename = "BLUFOR")]
    Blufor,

    #[serde(rename = "CIV")]
    Civ,

    #[serde(rename = "INDFOR")]
    Indfor,

    #[serde(rename = "OPFOR")]
    Opfor,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VehicleElement {
    pub label: Option<String>,

    pub vehicle: String,
}
