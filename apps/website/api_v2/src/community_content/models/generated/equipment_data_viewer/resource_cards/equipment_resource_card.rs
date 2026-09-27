// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/equipment-data-viewer/resource-cards.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::EquipmentSourceFact;

///`EquipmentResourceCard`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentResourceCard {
    pub capabilities: ::std::vec::Vec<::std::string::String>,
    pub class_name: ::std::string::String,
    pub facts: ::std::vec::Vec<EquipmentSourceFact>,
    pub index: u64,
    pub instance_name: ::std::string::String,
    pub metadata_json: ::std::string::String,
    pub node_id: ::std::string::String,
    pub property_count: u64,
    pub property_next_cursor: ::std::option::Option<::std::string::String>,
    pub view: ::std::string::String,
}
