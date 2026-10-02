//! Mirrors the generated equipment viewer API contract.
/// One page of source entries, as the `selection`, `containers`, `properties`, `values` and
/// `documents` endpoints return it: the entries, the node metadata JSON that describes them, the
/// total and the next page's cursor.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EquipmentSourcePage {
    pub dataset_kind: String,
    pub generation_id: ::std::string::String,
    pub items: ::std::vec::Vec<EquipmentSourcePageItemsItem>,
    pub next_cursor: ::std::option::Option<::std::string::String>,
    pub node_id: ::std::option::Option<::std::string::String>,
    pub node_metadata_json: ::std::string::String,
    pub resource_id: ::std::string::String,
    pub total: u64,
}
///`EquipmentSourcePageItemsItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
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
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EquipmentSourcePageItemsItemLinksItem {
    pub label: ::std::string::String,
    pub node_id: ::std::string::String,
    pub ordinal: ::std::option::Option<u64>,
    pub relationship: ::std::string::String,
}
