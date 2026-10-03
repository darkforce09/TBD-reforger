//! Bounded read queries pinned to one immutable generation.
pub mod fields;
pub mod native_matches;
pub mod relationships;
pub mod resource_cards;
pub mod resources;
pub mod selection;
pub mod source_inspection;
mod values;

/// The largest answer, in bytes, a read query returns.
pub const PAGE_BYTES: usize = 256 * 1024;

#[cfg(test)]
#[path = "../tests/value_expansion.rs"]
mod value_expansion_tests;

use crate::error::{Required, Result, ensure};
use api_identifiers::{
    EquipmentFieldId, EquipmentGenerationId, EquipmentNodeId, EquipmentResourceId,
};
use serde::Deserialize;
use serde_json::{Value, json};

/// The query parameters every equipment data viewer route reads; each route uses the ones it
/// needs.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ViewerQuery {
    /// The dataset kind, `gameplay` (the default) or `diagnostic`.
    pub dataset: Option<String>,
    /// The generation to answer from; `latest` (the default) is the current one.
    pub generation: Option<String>,
    /// The offset the page starts at, as the previous page's `next_cursor` spelled it.
    pub cursor: Option<String>,
    /// A case-insensitive search text.
    pub q: Option<String>,
    /// The equipment domain to filter on.
    pub domain: Option<String>,
    /// The capability to filter on.
    pub capability: Option<String>,
    /// The node view, `effective` (the default) or another the export recorded.
    pub view: Option<String>,
    /// The resource the route reads.
    pub resource_id: Option<EquipmentResourceId>,
    /// The node the route reads, or whose children it lists.
    pub node_id: Option<EquipmentNodeId>,
    /// The property to filter on.
    pub property: Option<String>,
    /// The field whose occurrences the route lists.
    pub field_id: Option<EquipmentFieldId>,
    /// The reference direction, forward or reverse.
    pub direction: Option<String>,
    /// A route-specific variant of the answer, such as `organized` or `native_type_match`.
    pub kind: Option<String>,
    /// The manifest path of the document to read or download.
    pub document: Option<String>,
    /// The JSON pointer into the document.
    pub pointer: Option<String>,
    /// The native class to filter on.
    pub class_name: Option<String>,
}
impl ViewerQuery {
    /// The page offset `cursor` spells (`0` when absent); above 100 000 000 it is refused.
    pub fn offset(&self) -> Result<usize> {
        let offset = self.cursor.as_deref().unwrap_or("0").parse::<usize>()?;
        ensure!(offset <= 100_000_000, "cursor is too large");
        Ok(offset)
    }
    /// The generation the query names, `latest` when absent.
    pub fn generation(&self) -> EquipmentGenerationId {
        EquipmentGenerationId::new(self.generation.as_deref().unwrap_or("latest"))
    }
    /// The resource the query names; a route that needs one refuses its absence.
    pub fn resource(&self) -> Result<&str> {
        self.resource_id
            .as_ref()
            .map(EquipmentResourceId::as_str)
            .required("resource_id is required")
    }
    /// The node view the query names, `effective` when absent.
    pub fn view(&self) -> &str {
        self.view.as_deref().unwrap_or("effective")
    }
}

/// One page answer of `items` from `offset` of `total`, trimmed to stay under [`PAGE_BYTES`]
/// with its `next_cursor` kept; a single row too large for a page is refused.
pub fn page(generation: &str, total: i64, offset: usize, items: Vec<Value>) -> Result<Value> {
    page_with_budget(generation, total, offset, items, PAGE_BYTES - 2048)
}

pub(super) fn page_with_budget(
    generation: &str,
    total: i64,
    offset: usize,
    mut items: Vec<Value>,
    budget: usize,
) -> Result<Value> {
    // Each row is independently expandable. Byte limiting must never drop the continuation.
    let mut bytes = 256usize;
    let mut length = 0;
    for item in &items {
        let size = serde_json::to_vec(item)?.len() + 1;
        if bytes + size > budget {
            break;
        }
        bytes += size;
        length += 1;
    }
    ensure!(
        items.is_empty() || length > 0,
        "row exceeds the response limit; open its original document"
    );
    items.truncate(length);
    let next = (offset + items.len() < total as usize).then(|| (offset + items.len()).to_string());
    Ok(json!({"generation_id":generation,"total":total,"next_cursor":next,"items":items}))
}
