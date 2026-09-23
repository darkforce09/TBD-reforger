// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-hub.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::MissionArmory;

///`ArmoryFaction`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ArmoryFaction {
    pub faction: ::std::string::String,
    pub items: ::std::vec::Vec<MissionArmory>,
}
