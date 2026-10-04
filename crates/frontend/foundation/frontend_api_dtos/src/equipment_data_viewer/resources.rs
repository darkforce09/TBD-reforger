//! Mirrors the generated equipment viewer API contract.
use crate::identifiers::{EquipmentGenerationId, EquipmentResourceId};
/// One page of the `resources` endpoint: resource catalog entries with the total and the next
/// page's cursor.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentResourcePage {
    /// The equipment dataset the page reads.
    pub dataset_kind: String,
    /// The export generation the page reads.
    pub generation_id: EquipmentGenerationId,
    /// The resources of this page.
    pub items: ::std::vec::Vec<EquipmentResourcePageItemsItem>,
    /// The cursor that fetches the next page; absent on the last page.
    pub next_cursor: ::std::option::Option<::std::string::String>,
    /// How many resources match.
    pub total: u64,
}
///`EquipmentResourcePageItemsItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentResourcePageItemsItem {
    /// The capabilities the resource carries.
    pub capabilities: ::std::vec::Vec<::std::string::String>,
    /// The equipment domains the resource belongs to.
    pub domains: ::std::vec::Vec<::std::string::String>,
    /// How many facts the resource holds.
    pub fact_count: u64,
    /// The resource's display label.
    pub label: ::std::string::String,
    /// How many nodes the resource's tree holds.
    pub node_count: u64,
    /// The resource's identity.
    pub resource_id: EquipmentResourceId,
    /// The resource's file name in the game data.
    pub resource_name: ::std::string::String,
    /// The addons the resource comes from.
    pub source_addons: ::std::vec::Vec<::std::string::String>,
}
