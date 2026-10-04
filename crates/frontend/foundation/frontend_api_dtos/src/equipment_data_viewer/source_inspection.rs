//! Mirrors the generated equipment viewer API contract.
use crate::identifiers::{EquipmentGenerationId, EquipmentNodeId, EquipmentResourceId};
/// One page of source entries, as the `selection`, `containers`, `properties`, `values` and
/// `documents` endpoints return it: the entries, the node metadata JSON that describes them, the
/// total and the next page's cursor.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EquipmentSourcePage {
    /// The equipment dataset the page reads.
    pub dataset_kind: String,
    /// The export generation the page reads.
    pub generation_id: EquipmentGenerationId,
    /// The source entries of this page.
    pub items: ::std::vec::Vec<EquipmentSourcePageItemsItem>,
    /// The cursor that fetches the next page; absent on the last page.
    pub next_cursor: ::std::option::Option<::std::string::String>,
    /// The node whose entries the page holds; absent at the resource root.
    pub node_id: ::std::option::Option<EquipmentNodeId>,
    /// The node's metadata, as JSON text.
    pub node_metadata_json: ::std::string::String,
    /// The resource the page inspects.
    pub resource_id: EquipmentResourceId,
    /// How many entries the node holds.
    pub total: u64,
}
///`EquipmentSourcePageItemsItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EquipmentSourcePageItemsItem {
    /// The native class to filter on.
    pub class_name: ::std::option::Option<::std::string::String>,
    /// What kind of entry this is.
    pub entry_kind: ::std::string::String,
    /// Whether the entry's children are expanded.
    pub expanded: bool,
    /// The entry's key within its node.
    pub key: ::std::string::String,
    /// The entry's display label.
    pub label: ::std::string::String,
    /// The nodes the entry links to.
    pub links: ::std::vec::Vec<EquipmentSourcePageItemsItemLinksItem>,
    /// The entry's metadata, as JSON text.
    pub metadata_json: ::std::string::String,
    /// The entry's Enfusion type; absent when untyped.
    pub native_type: ::std::option::Option<::std::string::String>,
    /// The unit the entry's value is given in; absent when unitless.
    pub native_unit: ::std::option::Option<::std::string::String>,
    /// The node the route reads, or whose children it lists.
    pub node_id: ::std::option::Option<EquipmentNodeId>,
    /// Where the entry's value comes from; absent when unknown.
    pub origin: ::std::option::Option<::std::string::String>,
    /// The entry's status; absent when none applies.
    pub status: ::std::option::Option<::std::string::String>,
    /// What the entry's unit was inferred from; absent when it was declared.
    pub unit_evidence: ::std::option::Option<::std::string::String>,
    /// How many values the entry holds.
    pub value_count: u64,
    /// The entry's value, as JSON text.
    pub value_json: ::std::string::String,
    /// What kind of value the entry holds.
    pub value_kind: ::std::string::String,
    /// The node view, `effective` (the default) or another the export recorded.
    pub view: ::std::option::Option<::std::string::String>,
}
///`EquipmentSourcePageItemsItemLinksItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EquipmentSourcePageItemsItemLinksItem {
    /// The linked node's display label.
    pub label: ::std::string::String,
    /// The linked node.
    pub node_id: EquipmentNodeId,
    /// The link's position among the entry's links; absent when unordered.
    pub ordinal: ::std::option::Option<u64>,
    /// How the entry relates to the linked node.
    pub relationship: ::std::string::String,
}
