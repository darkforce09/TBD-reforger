//! Mirrors the generated equipment viewer API contract.
/// One page of the `fields` endpoint: native fields with their occurrence counts, or the
/// occurrences of the selected field, with the total and the next page's cursor.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentFieldPage {
    pub dataset_kind: String,
    pub generation_id: ::std::string::String,
    pub items: ::std::vec::Vec<EquipmentFieldPageItemsItem>,
    pub next_cursor: ::std::option::Option<::std::string::String>,
    pub occurrences: ::std::vec::Vec<EquipmentFieldPageOccurrencesItem>,
    pub total: u64,
}
///`EquipmentFieldPageItemsItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentFieldPageItemsItem {
    pub aliases: ::std::vec::Vec<::std::string::String>,
    pub ancestor_count: u64,
    pub capabilities: ::std::vec::Vec<::std::string::String>,
    pub class_name: ::std::string::String,
    pub effective_count: u64,
    pub field_id: u64,
    pub native_type: ::std::string::String,
    pub property: ::std::string::String,
    pub resource_count: u64,
    pub units: ::std::vec::Vec<::std::string::String>,
}
///`EquipmentFieldPageOccurrencesItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentFieldPageOccurrencesItem {
    pub label: ::std::string::String,
    pub node_id: ::std::string::String,
    pub origin: ::std::string::String,
    pub property: ::std::string::String,
    pub resource_id: ::std::string::String,
    pub resource_name: ::std::string::String,
    pub status: ::std::string::String,
    pub view: ::std::string::String,
}
