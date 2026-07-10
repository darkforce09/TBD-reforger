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
/// (browse, seed/import, loadout build). v2 (T-150): kind vocabulary expanded for the
/// universal mod-agnostic scanner; optional addons[] records the Workbench scan set.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryItems {
    /// Workbench addons loaded during the export (the scan set). Optional for v1 envelopes; the
    /// universal exporter (T-150) always writes it.
    pub addons: Option<Vec<AddonElement>>,

    pub generated_at: Option<String>,

    pub items: Vec<ItemElement>,

    pub modpack_id: String,

    pub registry_items_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddonElement {
    /// Addon GUID from GameProject.GetLoadedAddons.
    pub guid: String,

    /// Addon ID (GameProject.GetAddonID), e.g. ArmaReforger.
    pub name: String,

    /// Human title (GameProject.GetAddonTitle).
    pub title: Option<String>,

    /// GameProject.IsVanillaAddon.
    pub vanilla: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemElement {
    /// Slash-delimited browse path, e.g. NATO/Rifleman.
    pub category: String,

    pub display_name: String,

    pub icon_url: Option<String>,

    /// v2 (T-150) classification. Phase 1 kinds (character, gear_primary, gear_uniform,
    /// gear_vest, gear_helmet) remain valid; 'other' is the escape hatch and its count must be
    /// reported in export verify logs.
    pub kind: Kind,

    /// Enfusion ResourceName ({GUID}Prefabs/.../File.et) used by Resource.Load.
    pub resource_name: String,
}

/// v2 (T-150) classification. Phase 1 kinds (character, gear_primary, gear_uniform,
/// gear_vest, gear_helmet) remain valid; 'other' is the escape hatch and its count must be
/// reported in export verify logs.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Ammo,

    Attachment,

    Character,

    Crate,

    #[serde(rename = "gear_backpack")]
    GearBackpack,

    #[serde(rename = "gear_handgun")]
    GearHandgun,

    #[serde(rename = "gear_helmet")]
    GearHelmet,

    #[serde(rename = "gear_launcher")]
    GearLauncher,

    #[serde(rename = "gear_primary")]
    GearPrimary,

    #[serde(rename = "gear_uniform")]
    GearUniform,

    #[serde(rename = "gear_vest")]
    GearVest,

    Magazine,

    Optic,

    Other,

    Vehicle,

    #[serde(rename = "vehicle_weapon")]
    VehicleWeapon,
}
