//! Mirrors the generated equipment viewer API contract.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentDatasetStatus {
    pub dataset_kind: String,
    pub completed: u64,
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
