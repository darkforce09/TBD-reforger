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
    /// True for non-placeable template prefabs (filename *_base.et / display '* Base'). Kept in
    /// the catalog (bases carry classification signals for descendants) but hidden from loadout
    /// pickers.
    #[serde(rename = "abstract")]
    pub registry_items_schema_abstract: Option<bool>,

    /// Per-item mod provenance: the addon ID this prefab was scanned from. Must match an
    /// addons[].name entry (strict check in validate.mjs); vanilla-ness derives from
    /// addons[].vanilla — no separate flag to drift.
    pub addon: Option<String>,

    /// SCR_EArsenalItemType flag name (e.g. RIFLE, NON_LETHAL_THROWABLE) when the item appears
    /// in a faction EntityCatalog SCR_ArsenalItem entry (Tier-B classification metadata). Absent
    /// when no catalog entry exists.
    pub arsenal_type: Option<String>,

    /// Slash-delimited browse path, e.g. NATO/Rifleman.
    pub category: String,

    pub display_name: String,

    pub icon_url: Option<String>,

    /// v3 (T-068.10.2) classification. Phase 1 kinds remain valid; gear_uniform is retired (0
    /// rows — split into gear_jacket/gear_pants/gear_boots) but still accepted; 'other' is the
    /// escape hatch and its count must be reported in export verify logs. Taxonomy:
    /// .ai/artifacts/ace_arsenal_taxonomy_map.md.
    pub kind: Kind,

    /// Container volume capacity (storage component MaxCumulativeVolume, cm³) for items that ARE
    /// containers. Absent when the prefab relies on the engine class default — never guessed.
    /// Feeds the later cargo-budget slice.
    pub max_volume_cm3: Option<f64>,

    /// Container carry capacity (storage component m_fMaxWeight, kg) for items that ARE
    /// containers (vests/backpacks/jackets). Absent when the prefab relies on the engine class
    /// default — never guessed. Feeds the later cargo-budget slice.
    pub max_weight_kg: Option<f64>,

    /// Enfusion ResourceName ({GUID}Prefabs/.../File.et) used by Resource.Load.
    pub resource_name: String,

    /// T-068.10.5: set on factory attachment/camo CONFIGURATIONS of a base weapon (same family
    /// prefix, magwell, attachment-slot-type set and mesh — only pre-mounted
    /// attachments/materials differ, e.g. 'Rifle AK74N 1P29' → 'Rifle AK74N'). Points at the
    /// immediate parent item (must exist in the envelope — strict check in validate.mjs).
    /// Pickers hide variant rows like abstracts; the census artifact
    /// t068_10_5_weapon_families.md carries the per-weapon evidence.
    pub variant_of: Option<String>,

    /// ItemPhysicalAttributes.ItemVolume in cubic centimetres (API-documented unit), read from
    /// the prefab ancestry chain. Absent when the value is an engine class default not
    /// serialized in the prefab — never guessed.
    pub volume_cm3: Option<f64>,

    /// ItemPhysicalAttributes.Weight in kilograms (API-documented unit), read from the prefab
    /// ancestry chain. Absent when the value is an engine class default not serialized in the
    /// prefab — never guessed.
    pub weight_kg: Option<f64>,
}

/// v3 (T-068.10.2) classification. Phase 1 kinds remain valid; gear_uniform is retired (0
/// rows — split into gear_jacket/gear_pants/gear_boots) but still accepted; 'other' is the
/// escape hatch and its count must be reported in export verify logs. Taxonomy:
/// .ai/artifacts/ace_arsenal_taxonomy_map.md.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Ammo,

    Attachment,

    Character,

    Crate,

    #[serde(rename = "gear_armored_vest")]
    GearArmoredVest,

    #[serde(rename = "gear_backpack")]
    GearBackpack,

    #[serde(rename = "gear_binoculars")]
    GearBinoculars,

    #[serde(rename = "gear_boots")]
    GearBoots,

    #[serde(rename = "gear_explosive")]
    GearExplosive,

    #[serde(rename = "gear_glasses")]
    GearGlasses,

    #[serde(rename = "gear_gloves")]
    GearGloves,

    #[serde(rename = "gear_handgun")]
    GearHandgun,

    #[serde(rename = "gear_helmet")]
    GearHelmet,

    #[serde(rename = "gear_item")]
    GearItem,

    #[serde(rename = "gear_jacket")]
    GearJacket,

    #[serde(rename = "gear_launcher")]
    GearLauncher,

    #[serde(rename = "gear_pants")]
    GearPants,

    #[serde(rename = "gear_primary")]
    GearPrimary,

    #[serde(rename = "gear_throwable")]
    GearThrowable,

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
