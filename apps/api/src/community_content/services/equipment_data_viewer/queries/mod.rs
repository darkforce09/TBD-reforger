//! Bounded read queries pinned to one immutable generation.
pub mod fields;
pub mod native_matches;
pub mod relationships;
pub mod resource_cards;
pub mod resources;
pub mod selection;
pub mod source_inspection;
mod values;

#[cfg(test)]
#[path = "../tests/value_expansion.rs"]
mod value_expansion_tests;

use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ViewerQuery {
    pub dataset: Option<String>,
    pub generation: Option<String>,
    pub cursor: Option<String>,
    pub q: Option<String>,
    pub domain: Option<String>,
    pub capability: Option<String>,
    pub view: Option<String>,
    pub resource_id: Option<String>,
    pub node_id: Option<String>,
    pub property: Option<String>,
    pub field_id: Option<i64>,
    pub direction: Option<String>,
    pub kind: Option<String>,
    pub document: Option<String>,
    pub pointer: Option<String>,
    pub class_name: Option<String>,
}
impl ViewerQuery {
    pub fn offset(&self) -> Result<usize> {
        let offset = self.cursor.as_deref().unwrap_or("0").parse::<usize>()?;
        ensure!(offset <= 100_000_000, "cursor is too large");
        Ok(offset)
    }
    pub fn generation(&self) -> &str {
        self.generation.as_deref().unwrap_or("latest")
    }
    pub fn resource(&self) -> Result<&str> {
        self.resource_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("resource_id is required"))
    }
    pub fn view(&self) -> &str {
        self.view.as_deref().unwrap_or("effective")
    }
}

pub fn page(generation: &str, total: i64, offset: usize, items: Vec<Value>) -> Result<Value> {
    page_with_budget(generation, total, offset, items, super::PAGE_BYTES - 2048)
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
