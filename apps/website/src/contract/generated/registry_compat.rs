// Code generated from JSON Schema using quicktype. DO NOT EDIT.
// Source: packages/tbd-schema/schema/registry-compat.schema.json — regenerate with: make schema-codegen

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::registry_compat;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: registry_compat = serde_json::from_str(&json).unwrap();
// }

use serde::{Deserialize, Serialize};

/// Engine-derived compatibility edge graph between registry items (T-150). Nodes are full
/// Enfusion ResourceNames and must exist in the paired registry-items envelope; edges are
/// read from prefab container data (magazine wells, attachment slot types, vehicle weapon
/// slots, character loadout slots) — never hand-authored. Drives canEquip/canAttach (T-068.9
/// ingest, T-068.10 smart Forge).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryCompat {
    /// Workbench addons loaded during the export (the scan set).
    pub addons: Option<Vec<AddonElement>>,

    pub edges: Vec<EdgeElement>,

    pub generated_at: Option<String>,

    pub modpack_id: String,

    pub registry_compat_version: String,
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

/// Directed compatibility edge. from_node = the item that goes in/on (magazine, ammo, optic,
/// attachment, gear); to_node = the host that accepts it (weapon, magazine, vehicle weapon,
/// character). Per edge_type: mag_in_weapon mag->weapon; ammo_in_mag ammo->magazine;
/// optic_on_weapon optic->weapon; attachment_on_weapon attachment->weapon;
/// mag_in_vehicle_weapon mag->vehicle weapon prefab; ammo_in_vehicle_weapon ammo->vehicle
/// weapon prefab; character_default_loadout gear item->character.
#[derive(Debug, Serialize, Deserialize)]
pub struct EdgeElement {
    pub edge_type: EdgeType,

    /// Engine class or container var that proved the edge, e.g. MagazineWellStanag556
    /// (well-class match), AttachmentOpticsRIS1913 (slot type match), MagazineTemplate (direct
    /// prefab ref), LoadoutSlotInfo (character slot).
    pub evidence: Option<String>,

    pub from_node: String,

    pub to_node: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    #[serde(rename = "ammo_in_mag")]
    AmmoInMag,

    #[serde(rename = "ammo_in_vehicle_weapon")]
    AmmoInVehicleWeapon,

    #[serde(rename = "attachment_on_weapon")]
    AttachmentOnWeapon,

    #[serde(rename = "character_default_loadout")]
    CharacterDefaultLoadout,

    #[serde(rename = "character_default_weapon")]
    CharacterDefaultWeapon,

    #[serde(rename = "mag_in_vehicle_weapon")]
    MagInVehicleWeapon,

    #[serde(rename = "mag_in_weapon")]
    MagInWeapon,

    #[serde(rename = "optic_on_weapon")]
    OpticOnWeapon,
}
