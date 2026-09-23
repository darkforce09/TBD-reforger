// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/registry-compat.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{Addon, Edge};

///Engine-derived compatibility edge graph between registry items. Nodes are full Enfusion ResourceNames and must exist in the paired registry-items envelope; edges are read from prefab container data (magazine wells, attachment slot types, vehicle weapon slots, character loadout slots) and are never hand-authored. Drives the canEquip / canAttach answers the Arsenal and the smart Forge rely on.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TbdRegistryCompat {
    ///Workbench addons loaded during the export (the scan set).
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub addons: ::std::vec::Vec<Addon>,
    pub edges: ::std::vec::Vec<Edge>,
    #[serde(
        rename = "generatedAt",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub generated_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    #[serde(rename = "modpackId")]
    pub modpack_id: ::std::string::String,
    #[serde(rename = "registryCompatVersion")]
    pub registry_compat_version: ::std::string::String,
}
