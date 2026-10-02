//! Mirrors the generated equipment viewer API contract.
/// One page of the `resources` endpoint: resource catalog entries with the total and the next
/// page's cursor.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentResourcePage {
    pub dataset_kind: String,
    pub generation_id: ::std::string::String,
    pub items: ::std::vec::Vec<EquipmentResourcePageItemsItem>,
    pub next_cursor: ::std::option::Option<::std::string::String>,
    pub total: u64,
}
///`EquipmentResourcePageItemsItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentResourcePageItemsItem {
    pub capabilities: ::std::vec::Vec<::std::string::String>,
    pub domains: ::std::vec::Vec<::std::string::String>,
    pub fact_count: u64,
    pub label: ::std::string::String,
    pub node_count: u64,
    pub resource_id: ::std::string::String,
    pub resource_name: ::std::string::String,
    pub source_addons: ::std::vec::Vec<::std::string::String>,
}
