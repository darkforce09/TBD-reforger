//! Batched source cards preserve native container order and source value tokens.
use super::super::{Dataset, EquipmentDataService, PAGE_BYTES, source::document::Snapshot};
use super::{ViewerQuery, source_inspection::property_entry};
use crate::error::{Required, Result, ensure};
use serde_json::{Value, json};
use sqlx::Row;
use std::{collections::BTreeMap, sync::Arc};

/// The queried resource's source cards: each capability node with its facts, in native container
/// order and with the source value tokens, batched to stay under [`PAGE_BYTES`].
pub async fn list(
    service: Arc<EquipmentDataService>,
    dataset: Arc<Dataset>,
    query: ViewerQuery,
) -> Result<Value> {
    let _permit = service.readers.clone().acquire_owned().await?;
    let resource = query.resource()?;
    let file: String = sqlx::query_scalar("SELECT source_file FROM resources WHERE resource_id=?")
        .bind(resource)
        .fetch_optional(&dataset.pool)
        .await?
        .required("resource not found")?;
    let rows = sqlx::query("SELECT n.node_id,c.capability FROM capabilities c JOIN nodes n ON n.id=c.node JOIN resources r ON r.id=n.resource WHERE r.resource_id=? ORDER BY c.capability")
        .bind(resource).fetch_all(&dataset.pool).await?;
    let mut capabilities = BTreeMap::<String, Vec<String>>::new();
    for row in rows {
        capabilities
            .entry(row.get("node_id"))
            .or_default()
            .push(row.get("capability"));
    }
    let bytes = service.document(&dataset, &file).await?;
    tokio::task::spawn_blocking(move || {
        build_snapshot(
            super::super::source::gameplay::snapshot(&bytes, &dataset.definitions)?,
            dataset.generation_id.as_str(),
            &query,
            &capabilities,
        )
    })
    .await?
}

#[cfg(test)]
pub(super) fn build(
    bytes: &[u8],
    generation: &str,
    query: &ViewerQuery,
    capabilities: &BTreeMap<String, Vec<String>>,
) -> Result<Value> {
    build_snapshot(
        serde_json::from_slice(bytes)?,
        generation,
        query,
        capabilities,
    )
}

fn build_snapshot(
    snapshot: Snapshot,
    generation: &str,
    query: &ViewerQuery,
    capabilities: &BTreeMap<String, Vec<String>>,
) -> Result<Value> {
    let search = query.q.as_deref().unwrap_or("").to_lowercase();
    let nodes: Vec<_> = snapshot
        .nodes
        .iter()
        .filter(|n| query.view() == "all" || n.view == query.view())
        .filter(|n| {
            search.is_empty()
                || n.class_name.to_lowercase().contains(&search)
                || n.instance_name.to_lowercase().contains(&search)
                || n.properties
                    .keys()
                    .any(|p| p.to_lowercase().contains(&search))
        })
        .collect();
    let offset = if let Some(id) = &query.node_id {
        let index = nodes
            .iter()
            .position(|n| &n.node_id == id)
            .required("source container is absent from this view")?;
        index / 12 * 12
    } else {
        query.offset()?
    };
    let mut items = Vec::new();

    for (index, node) in nodes.iter().enumerate().skip(offset).take(12) {
        let mut facts = Vec::new();
        if query.kind.as_deref() != Some("contents") {
            for (property, fact) in node.properties.iter().take(12) {
                let mut entry = property_entry(node, property, fact, None)?;
                if entry["metadata_json"]
                    .as_str()
                    .is_some_and(|s| s.len() > 4096)
                {
                    entry["metadata_json"] = json!("{\"expand_metadata\":true}");
                }
                facts.push(entry);
            }
        }
        let mut metadata = node.metadata().to_string();
        if metadata.len() > 8192 {
            metadata = json!({"node_id":node.node_id,"expand_metadata":true}).to_string();
        }
        let mut card = json!({"node_id":node.node_id,"class_name":node.class_name,"instance_name":node.instance_name,"view":node.view,"index":index,"capabilities":capabilities.get(node.node_id.as_str()).cloned().unwrap_or_default(),"property_count":node.properties.len(),"property_next_cursor":null,"metadata_json":metadata,"facts":facts});
        loop {
            let length = card["facts"]
                .as_array()
                .expect("a card's facts are an array")
                .len();
            card["property_next_cursor"] = if length < node.properties.len() {
                json!(length.to_string())
            } else {
                Value::Null
            };
            let size = serde_json::to_vec(&card)?.len() + 1;
            if size < (PAGE_BYTES - 4096) / 12 {
                break;
            }
            if length > 0 {
                card["facts"]
                    .as_array_mut()
                    .expect("a card's facts are an array")
                    .pop();
            } else {
                break;
            }
        }
        items.push(card);
    }
    let next = (offset + items.len() < nodes.len()).then(|| (offset + items.len()).to_string());
    let result = json!({"generation_id":generation,"resource_id":snapshot.resource_id,"total":nodes.len(),"start_index":offset,"next_cursor":next,"items":items});
    ensure!(
        serde_json::to_vec(&result)?.len() <= PAGE_BYTES,
        "source card metadata exceeds response limit"
    );
    Ok(result)
}

#[cfg(test)]
#[path = "../tests/resource_cards.rs"]
mod tests;
