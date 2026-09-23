// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/registry-items.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{Addon, Item};

///Flat catalog of placeable and equipable engine items exported from the TBD-Content Workbench. Items are identified by their full Enfusion ResourceName (`resource_name`). This is a separate layer from the alias spawn registry in `registry.schema.json`: that registry maps mission aliases to GUIDs for spawn, while this catalog drives the web Virtual Arsenal (browse, seed / import, loadout build). A v2 envelope widens the `kind` vocabulary for the mod-agnostic scanner and carries an optional `addons[]` recording the Workbench scan set.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TbdRegistryItems {
    ///Workbench addons loaded during the export — the scan set. Optional in a v1 envelope; the mod-agnostic exporter always writes it.
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub addons: ::std::vec::Vec<Addon>,
    #[serde(
        rename = "generatedAt",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub generated_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    pub items: ::std::vec::Vec<Item>,
    #[serde(rename = "modpackId")]
    pub modpack_id: ::std::string::String,
    #[serde(rename = "registryItemsVersion")]
    pub registry_items_version: ::std::string::String,
}
