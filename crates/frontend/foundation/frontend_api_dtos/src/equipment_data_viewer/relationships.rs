//! Mirrors the generated equipment viewer API contract.
use crate::identifiers::{EquipmentGenerationId, EquipmentNodeId, EquipmentResourceId};
/// One page of the `relationships` endpoint: the selected resource's reference occurrences in
/// one direction, with the total and the next page's cursor.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentRelationshipPage {
    /// The equipment dataset the page reads.
    pub dataset_kind: String,
    /// The export generation the page reads.
    pub generation_id: EquipmentGenerationId,
    /// The relationships of this page.
    pub items: ::std::vec::Vec<EquipmentRelationshipPageItemsItem>,
    /// The cursor that fetches the next page; absent on the last page.
    pub next_cursor: ::std::option::Option<::std::string::String>,
    /// How many relationships match.
    pub total: u64,
}
///`EquipmentRelationshipPageItemsItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentRelationshipPageItemsItem {
    /// What the relationship was derived from, as JSON text; absent when declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_json: Option<String>,
    /// A route-specific variant of the answer, such as `organized` or `native_type_match`.
    pub kind: ::std::string::String,
    /// How the export resolved the reference.
    pub method: ::std::string::String,
    /// The node the route reads, or whose children it lists.
    pub node_id: EquipmentNodeId,
    /// The property to filter on.
    pub property: ::std::string::String,
    /// The resource the route reads.
    pub resource_id: EquipmentResourceId,
    /// The resource named.
    pub resource_name: ::std::string::String,
    /// The related resource; absent when it is outside the dataset.
    pub target_resource_id: ::std::option::Option<EquipmentResourceId>,
    /// The related resource's name.
    pub target_resource_name: ::std::string::String,
    /// The node view, `effective` (the default) or another the export recorded.
    pub view: ::std::string::String,
}
