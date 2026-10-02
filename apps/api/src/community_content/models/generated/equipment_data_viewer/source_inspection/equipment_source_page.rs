// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/equipment-data-viewer/source-inspection.schema.json — regenerate with: cargo xtask ci schema-codegen

///`EquipmentSourcePage`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentSourcePage {
    pub dataset_kind: ::std::string::String,
    pub generation_id: ::std::string::String,
    pub items: ::std::vec::Vec<EquipmentSourcePageItemsItem>,
    pub next_cursor: ::std::option::Option<::std::string::String>,
    pub node_id: ::std::option::Option<::std::string::String>,
    pub node_metadata_json: ::std::string::String,
    pub resource_id: ::std::string::String,
    pub total: u64,
}
///`EquipmentSourcePageItemsItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentSourcePageItemsItem {
    pub class_name: ::std::option::Option<::std::string::String>,
    pub entry_kind: ::std::string::String,
    pub expanded: bool,
    pub key: ::std::string::String,
    pub label: ::std::string::String,
    pub links: ::std::vec::Vec<EquipmentSourcePageItemsItemLinksItem>,
    pub metadata_json: ::std::string::String,
    pub native_type: ::std::option::Option<::std::string::String>,
    pub native_unit: ::std::option::Option<::std::string::String>,
    pub node_id: ::std::option::Option<::std::string::String>,
    pub origin: ::std::option::Option<::std::string::String>,
    pub status: ::std::option::Option<::std::string::String>,
    pub unit_evidence: ::std::option::Option<::std::string::String>,
    pub value_count: u64,
    pub value_json: ::std::string::String,
    pub value_kind: ::std::string::String,
    pub view: ::std::option::Option<::std::string::String>,
}
///`EquipmentSourcePageItemsItemLinksItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentSourcePageItemsItemLinksItem {
    pub label: ::std::string::String,
    pub node_id: ::std::string::String,
    pub ordinal: ::std::option::Option<u64>,
    pub relationship: ::std::string::String,
}
