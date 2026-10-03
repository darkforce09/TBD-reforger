//! Mirrors the generated equipment viewer API contract.
/// One page of the `relationships` endpoint: the selected resource's reference occurrences in
/// one direction, with the total and the next page's cursor.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentRelationshipPage {
    pub dataset_kind: String,
    pub generation_id: ::std::string::String,
    pub items: ::std::vec::Vec<EquipmentRelationshipPageItemsItem>,
    pub next_cursor: ::std::option::Option<::std::string::String>,
    pub total: u64,
}
///`EquipmentRelationshipPageItemsItem`
#[cfg(any(target_arch = "wasm32", test))]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentRelationshipPageItemsItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_json: Option<String>,
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
