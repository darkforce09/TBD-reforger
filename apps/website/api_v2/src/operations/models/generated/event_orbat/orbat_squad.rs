// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-orbat.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::OrbatSlot;

///`OrbatSquad`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct OrbatSquad {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub callsign: ::std::option::Option<::std::string::String>,
    pub faction: ::std::string::String,
    pub filled: u64,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub reserved_by: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub reserved_by_name: ::std::option::Option<::std::string::String>,
    pub slots: ::std::vec::Vec<OrbatSlot>,
    pub squad: ::std::string::String,
    pub total: u64,
}
