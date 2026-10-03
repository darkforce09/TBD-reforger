// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/equipment-data-viewer/relationships.schema.json — regenerate with: cargo xtask ci schema-codegen

///`EquipmentRelationshipPage`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentRelationshipPage {
    pub dataset_kind: ::std::string::String,
    pub generation_id: ::std::string::String,
    pub items: ::std::vec::Vec<EquipmentRelationshipPageItemsItem>,
    pub next_cursor: ::std::option::Option<::std::string::String>,
    pub total: u64,
}
///`EquipmentRelationshipPageItemsItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentRelationshipPageItemsItem {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub evidence_json: ::std::option::Option<::std::string::String>,
    pub kind: ::std::string::String,
    pub method: ::std::string::String,
    pub node_id: ::std::string::String,
    pub property: ::std::string::String,
    pub resource_id: ::std::string::String,
    pub resource_name: ::std::string::String,
    pub target_resource_id: ::std::option::Option<::std::string::String>,
    pub target_resource_name: ::std::string::String,
    pub view: ::std::string::String,
}
