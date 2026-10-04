//! Mirrors the generated equipment viewer API contract.
use crate::identifiers::{
    EquipmentFieldId, EquipmentGenerationId, EquipmentNodeId, EquipmentResourceId,
};
/// One page of the `fields` endpoint: native fields with their occurrence counts, or the
/// occurrences of the selected field, with the total and the next page's cursor.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentFieldPage {
    /// The equipment dataset the page reads.
    pub dataset_kind: String,
    /// The export generation the page reads.
    pub generation_id: EquipmentGenerationId,
    /// The fields of this page.
    pub items: ::std::vec::Vec<EquipmentFieldPageItemsItem>,
    /// The cursor that fetches the next page; absent on the last page.
    pub next_cursor: ::std::option::Option<::std::string::String>,
    /// Where the selected field occurs.
    pub occurrences: ::std::vec::Vec<EquipmentFieldPageOccurrencesItem>,
    /// How many fields match.
    pub total: u64,
}
///`EquipmentFieldPageItemsItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentFieldPageItemsItem {
    /// The other names the field goes by.
    pub aliases: ::std::vec::Vec<::std::string::String>,
    /// How many ancestor classes declare the field.
    pub ancestor_count: u64,
    /// The capabilities of the resources holding the field.
    pub capabilities: ::std::vec::Vec<::std::string::String>,
    /// The native class to filter on.
    pub class_name: ::std::string::String,
    /// How many resources hold an effective value for the field.
    pub effective_count: u64,
    /// The field whose occurrences the route lists.
    pub field_id: EquipmentFieldId,
    /// The field's Enfusion type.
    pub native_type: ::std::string::String,
    /// The property to filter on.
    pub property: ::std::string::String,
    /// How many resources hold the field.
    pub resource_count: u64,
    /// The units the field's values are given in.
    pub units: ::std::vec::Vec<::std::string::String>,
}
///`EquipmentFieldPageOccurrencesItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EquipmentFieldPageOccurrencesItem {
    /// The label of the resource the field occurs in.
    pub label: ::std::string::String,
    /// The node the route reads, or whose children it lists.
    pub node_id: EquipmentNodeId,
    /// Where the occurrence's value comes from.
    pub origin: ::std::string::String,
    /// The property to filter on.
    pub property: ::std::string::String,
    /// The resource the route reads.
    pub resource_id: EquipmentResourceId,
    /// The resource file the object is declared in.
    pub resource_name: ::std::string::String,
    /// The occurrence's status.
    pub status: ::std::string::String,
    /// The node view, `effective` (the default) or another the export recorded.
    pub view: ::std::string::String,
}
