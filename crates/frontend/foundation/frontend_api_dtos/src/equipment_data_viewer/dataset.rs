//! Mirrors the generated equipment viewer API contract.
use crate::identifiers::EquipmentGenerationId;
/// The status of one equipment dataset (`gameplay` or `diagnostic`), as the `status` and
/// `overview` endpoints return it: the import stage with its progress and message, the ready
/// generation, the overview counts, and a cursor-paged list of preserved generations.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentDatasetStatus {
    /// The equipment dataset the status describes.
    pub dataset_kind: String,
    /// Units of the stage done.
    pub completed: u64,
    /// The export generation in force; absent before the first import.
    pub generation_id: ::std::option::Option<EquipmentGenerationId>,
    /// The imported export generations.
    pub generations: ::std::vec::Vec<EquipmentGenerationId>,
    /// The failure of the last import with its causes, or a note on the stage.
    pub message: ::std::option::Option<::std::string::String>,
    /// The cursor that fetches the next page of generations; absent on the last page.
    pub next_cursor: ::std::option::Option<::std::string::String>,
    /// The counts of the generation in force; absent before the first import.
    pub overview: ::std::option::Option<EquipmentDatasetStatusOverview>,
    /// The import stage (`starting`, `ready`, or the step in progress).
    pub stage: ::std::string::String,
    /// Units of the stage in all; `0` when the stage counts none.
    pub total: u64,
}
///`EquipmentDatasetStatusOverview`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentDatasetStatusOverview {
    /// The size of the generation's data, in bytes.
    pub bytes: u64,
    /// How many resources carry each capability.
    pub capabilities: ::std::collections::HashMap<::std::string::String, u64>,
    /// How many addon dependencies the generation records.
    pub dependencies: u64,
    /// How many equipment resources the generation holds.
    pub equipment: u64,
    /// How many facts the generation holds.
    pub facts: u64,
    /// How many distinct fields the generation holds.
    pub fields: u64,
    /// How many nodes the generation holds.
    pub nodes: u64,
    /// How many resources the generation holds.
    pub resources: u64,
    /// How many vehicle resources the generation holds.
    pub vehicles: u64,
}
