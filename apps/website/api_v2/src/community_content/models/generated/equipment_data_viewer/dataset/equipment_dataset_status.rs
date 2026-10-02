// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/equipment-data-viewer/dataset.schema.json — regenerate with: cargo xtask ci schema-codegen

///`EquipmentDatasetStatus`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentDatasetStatus {
    pub completed: u64,
    pub dataset_kind: ::std::string::String,
    pub generation_id: ::std::option::Option<::std::string::String>,
    pub generations: ::std::vec::Vec<::std::string::String>,
    pub message: ::std::option::Option<::std::string::String>,
    pub next_cursor: ::std::option::Option<::std::string::String>,
    pub overview: ::std::option::Option<EquipmentDatasetStatusOverview>,
    pub stage: ::std::string::String,
    pub total: u64,
}
///`EquipmentDatasetStatusOverview`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentDatasetStatusOverview {
    pub bytes: u64,
    pub capabilities: ::std::collections::HashMap<::std::string::String, u64>,
    pub dependencies: u64,
    pub equipment: u64,
    pub facts: u64,
    pub fields: u64,
    pub nodes: u64,
    pub resources: u64,
    pub vehicles: u64,
}
