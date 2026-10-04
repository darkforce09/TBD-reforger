//! Batched native container cards mirror the generated viewer contract.
use super::EquipmentSourcePageItemsItem;
use crate::identifiers::{EquipmentGenerationId, EquipmentNodeId, EquipmentResourceId};
/// One page of the `resource-cards` endpoint: the native container cards of one resource from
/// position `start_index`, with the container total and the next page's cursor.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquipmentResourceCardPage {
    /// The equipment dataset the page reads.
    pub dataset_kind: String,
    /// The export generation the page reads.
    pub generation_id: EquipmentGenerationId,
    /// The resource whose cards the page holds.
    pub resource_id: EquipmentResourceId,
    /// How many cards the resource has.
    pub total: u64,
    /// The index of the page's first card.
    pub start_index: u64,
    /// The cursor that fetches the next page; absent on the last page.
    pub next_cursor: Option<String>,
    /// The cards of this page.
    pub items: Vec<EquipmentResourceCard>,
}
/// One native container of a resource: its identity, configuration view, position, capabilities
/// and metadata JSON, with its first page of field facts and the cursor of the next.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquipmentResourceCard {
    /// The node's identity within its generation.
    pub node_id: EquipmentNodeId,
    /// The native class of the object.
    pub class_name: String,
    /// The object's instance name; empty when unnamed.
    pub instance_name: String,
    /// The view the node belongs to (`effective` for the resolved tree).
    pub view: String,
    /// The card's position within the resource.
    pub index: u64,
    /// The capabilities the card's node carries.
    pub capabilities: Vec<String>,
    /// How many properties the card has.
    pub property_count: u64,
    /// The cursor that fetches the card's further properties; absent when all are here.
    pub property_next_cursor: Option<String>,
    /// The card node's metadata, as JSON text.
    pub metadata_json: String,
    /// The card's first properties.
    pub facts: Vec<EquipmentSourcePageItemsItem>,
}
