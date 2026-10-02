//! Batched native container cards mirror the generated viewer contract.
use super::EquipmentSourcePageItemsItem;
/// One page of the `resource-cards` endpoint: the native container cards of one resource from
/// position `start_index`, with the container total and the next page's cursor.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquipmentResourceCardPage {
    pub dataset_kind: String,
    pub generation_id: String,
    pub resource_id: String,
    pub total: u64,
    pub start_index: u64,
    pub next_cursor: Option<String>,
    pub items: Vec<EquipmentResourceCard>,
}
/// One native container of a resource: its identity, configuration view, position, capabilities
/// and metadata JSON, with its first page of field facts and the cursor of the next.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquipmentResourceCard {
    pub node_id: String,
    pub class_name: String,
    pub instance_name: String,
    pub view: String,
    pub index: u64,
    pub capabilities: Vec<String>,
    pub property_count: u64,
    pub property_next_cursor: Option<String>,
    pub metadata_json: String,
    pub facts: Vec<EquipmentSourcePageItemsItem>,
}
