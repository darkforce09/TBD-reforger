// Code generated from JSON Schema using quicktype. DO NOT EDIT.
// Source: packages/tbd-schema/schema/registry-items.schema.json — regenerate with: make schema-codegen

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::registry_items;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: registry_items = serde_json::from_str(&json).unwrap();
// }

use serde::{Deserialize, Serialize};

/// Flat catalog of placeable/equipable engine items exported from the TBD-Content Workbench.
/// Items are identified by their full Enfusion ResourceName (resource_name). This is a
/// separate layer from the alias spawn registry (registry.schema.json): the alias registry
/// maps mission aliases to GUIDs for spawn, this catalog drives the web Virtual Arsenal
/// (browse, seed/import, loadout build).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryItems {
    pub generated_at: Option<String>,

    pub items: Vec<RegistryItemsSchema>,

    pub modpack_id: String,

    pub registry_items_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryItemsSchema {
    /// Slash-delimited browse path, e.g. NATO/Rifleman.
    pub category: String,

    pub display_name: String,

    pub icon_url: Option<String>,

    pub kind: Kind,

    /// Enfusion ResourceName ({GUID}Prefabs/.../File.et) used by Resource.Load.
    pub resource_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Character,

    #[serde(rename = "gear_helmet")]
    GearHelmet,

    #[serde(rename = "gear_primary")]
    GearPrimary,

    #[serde(rename = "gear_uniform")]
    GearUniform,

    #[serde(rename = "gear_vest")]
    GearVest,
}
