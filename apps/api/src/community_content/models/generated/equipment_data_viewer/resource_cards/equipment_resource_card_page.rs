// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/equipment-data-viewer/resource-cards.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::EquipmentResourceCard;

///`EquipmentResourceCardPage`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentResourceCardPage {
    pub dataset_kind: ::std::string::String,
    pub generation_id: ::std::string::String,
    pub items: ::std::vec::Vec<EquipmentResourceCard>,
    pub next_cursor: ::std::option::Option<::std::string::String>,
    pub resource_id: ::std::string::String,
    pub start_index: u64,
    pub total: u64,
}
