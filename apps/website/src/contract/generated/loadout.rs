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

/// Loadout download: a fixed set of gear slots, each holding a resource_name (from
/// registry-items) or null when empty. Consumed by the mod equip test and the web download.
/// optic/magazine are optional Smart Forge slots (T-068.10) — absent in Phase 1 payloads,
/// ignored by the v1 mod reader.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Loadout {
    pub gear: Gear,

    pub loadout_version: LoadoutVersion,

    pub modpack_id: String,
}

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
}
