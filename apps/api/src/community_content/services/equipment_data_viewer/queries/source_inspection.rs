//! Instance navigation, native facts and complete document expansion.
use super::super::{
    Dataset, EquipmentDataService,
    source::{
        document::{Fact, Node, Snapshot},
        gameplay, manifest,
    },
};
use super::{ViewerQuery, page_with_budget, values};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json, value::RawValue};
use sqlx::{QueryBuilder, Row, Sqlite};
use std::sync::Arc;

pub async fn nodes(d: &Dataset, p: &ViewerQuery) -> Result<Value> {
    let offset = p.offset()?;
    let resource = p.resource()?;
    let from = if p.node_id.is_some() {
        " FROM edges e JOIN nodes parent ON parent.id=e.node JOIN nodes n ON n.id=e.target JOIN resources r ON r.id=n.resource"
    } else {
        " FROM nodes n JOIN resources r ON r.id=n.resource"
    };
    let filters = |q: &mut QueryBuilder<Sqlite>| {
        q.push(" WHERE r.resource_id=")
            .push_bind(resource.to_owned());
        if let Some(parent) = &p.node_id {
            q.push(" AND parent.node_id=").push_bind(parent.clone());
        }
        if p.view() != "all" {
            q.push(" AND n.view=").push_bind(p.view().to_owned());
        }
        if let Some(cap) = p.capability.as_ref().filter(|s| !s.is_empty()) {
            q.push(" AND EXISTS(SELECT 1 FROM capabilities c WHERE c.node=n.id AND c.capability=")
                .push_bind(cap.clone())
                .push(")");
        }
        if p.node_id.is_none()
            && p.capability.as_deref().is_none_or(str::is_empty)
            && p.kind.as_deref() != Some("all")
        {
            q.push(" AND n.ordinal=-1");
        }
    };
    let mut count = QueryBuilder::new(format!("SELECT COUNT(*){from}"));
    filters(&mut count);
    let total: i64 = count.build_query_scalar().fetch_one(&d.pool).await?;
    let extras = if p.node_id.is_some() {
        ",e.relationship,e.property,e.ordinal edge_ordinal"
    } else {
        ""
    };
    let mut query = QueryBuilder::new(format!("SELECT n.*{extras}{from}"));
    filters(&mut query);
    query
        .push(if p.node_id.is_some() {
            " ORDER BY e.rowid LIMIT 100 OFFSET "
        } else {
            " ORDER BY n.ordinal LIMIT 100 OFFSET "
        })
        .push_bind(offset as i64);
    let rows = query.build().fetch_all(&d.pool).await?;
    let mut items = Vec::new();
    for row in rows {
        let id: String = row.get("node_id");
        let class: String = row.get("class_name");
        let instance: String = row.get("instance_name");
        let label = if p.node_id.is_some() {
            format!(
                "{} [{}] · {}",
                row.get::<String, _>("property"),
                row.get::<i64, _>("edge_ordinal"),
                class
            )
        } else if instance.is_empty() {
            class.clone()
        } else {
            format!("{class} · {instance}")
        };
        let mut item = values::entry(&id, &RawValue::from_string("null".into())?)?;
        item["entry_kind"] = json!("node");
        item["label"] = json!(label);
        item["node_id"] = json!(id);
        item["class_name"] = json!(class);
        item["view"] = json!(row.get::<String, _>("view"));
        item["metadata_json"]=json!(json!({"native_instance_id":row.get::<Option<String>,_>("native_instance_id"),"instance_identity_kind":row.get::<String,_>("identity_kind"),"instance_name":instance}).to_string());
        items.push(item);
    }
    source_page(d, p, total, offset, items, "{}")
}

pub async fn inspect(
    service: Arc<EquipmentDataService>,
    d: Arc<Dataset>,
    p: ViewerQuery,
    operation: &str,
) -> Result<Value> {
    let _permit = service.readers.clone().acquire_owned().await?;
    if operation == "document" {
        return document(service, d, p).await;
    }
    let resource = p.resource()?;
    let file: String = sqlx::query_scalar("SELECT source_file FROM resources WHERE resource_id=?")
        .bind(resource)
        .fetch_optional(&d.pool)
        .await?
        .context("resource not found")?;
    let bytes = service.document(&d, &file).await?;
    let organized = if p.kind.as_deref() == Some("organized") {
        let file: String =
            sqlx::query_scalar("SELECT record_file FROM resources WHERE resource_id=?")
                .bind(resource)
                .fetch_one(&d.pool)
                .await?;
        Some(service.document(&d, &file).await?)
    } else {
        None
    };
    let operation = operation.to_owned();
    tokio::task::spawn_blocking(move || -> Result<Value> {
        let snapshot = gameplay::snapshot(&bytes, &d.definitions)?;
        let id = p.node_id.as_deref().unwrap_or(&snapshot.root_node);
        let node = snapshot
            .nodes
            .iter()
            .find(|n| n.node_id == id)
            .context("source node not found")?;
        let aliases = organized
            .as_ref()
            .map(|bytes| {
                organized_aliases(bytes, &snapshot, p.capability.as_deref().unwrap_or(""), id)
            })
            .transpose()?;
        let offset = p.offset()?;
        if operation == "values" {
            let property = p.property.as_deref().context("property is required")?;
            let fact = node
                .properties
                .get(property)
                .context("source property not found")?;
            let raw = if p.kind.as_deref() == Some("metadata") {
                RawValue::from_string(serde_json::to_string(&fact.metadata)?)?
            } else {
                RawValue::from_string(fact.value.get().to_owned())?
            };
            let (total, items) = values::expand(&raw, p.pointer.as_deref().unwrap_or(""), offset)?;
            return source_page(
                &d,
                &p,
                total as i64,
                offset,
                items,
                &node.metadata().to_string(),
            );
        }
        let properties: Vec<_> = node
            .properties
            .iter()
            .filter(|(name, _)| aliases.as_ref().is_none_or(|a| a.contains_key(*name)))
            .filter(|(name, _)| p.property.as_ref().is_none_or(|selected| selected == *name))
            .filter(|(name, _)| {
                p.q.as_ref()
                    .is_none_or(|q| name.to_lowercase().contains(&q.to_lowercase()))
            })
            .collect();
        let mut items = Vec::new();
        for (property, fact) in properties.iter().skip(offset).take(100) {
            let item = property_entry(
                node,
                property,
                fact,
                aliases.as_ref().and_then(|a| a.get(*property)),
            )?;
            items.push(item);
        }
        source_page(
            &d,
            &p,
            properties.len() as i64,
            offset,
            items,
            &node.metadata().to_string(),
        )
    })
    .await?
}

fn organized_aliases(
    bytes: &[u8],
    snapshot: &Snapshot,
    capability: &str,
    node: &str,
) -> Result<std::collections::BTreeMap<String, Vec<String>>> {
    let record = gameplay::record(bytes, snapshot)?;
    let instances = record["capabilities"][capability]
        .as_array()
        .context("capability not found")?;
    let mut aliases = std::collections::BTreeMap::<String, Vec<String>>::new();
    for instance in instances
        .iter()
        .filter(|i| i["node_id"].as_str() == Some(node))
    {
        for (alias, fact) in instance["facts"]
            .as_object()
            .context("capability facts missing")?
        {
            let property = fact["source"]["property"]
                .as_str()
                .context("source property missing")?;
            aliases
                .entry(property.to_owned())
                .or_default()
                .push(alias.clone());
        }
    }
    Ok(aliases)
}

async fn document(
    service: Arc<EquipmentDataService>,
    d: Arc<Dataset>,
    p: ViewerQuery,
) -> Result<Value> {
    let kind = p.document.as_deref().unwrap_or("generation");
    let bytes = match kind {
        "generation" => service.document(&d, "generation.json").await?,
        "manifest" => Arc::new(manifest::read(&d.root, "manifest.json")?),
        "field_definitions"
        | "native_types"
        | "selection_report"
        | "resource_index"
        | "publication_receipt" => service.document(&d, &format!("{kind}.json")).await?,
        "record" | "source" => {
            let statement = if kind == "record" {
                "SELECT record_file FROM resources WHERE resource_id=?"
            } else {
                "SELECT source_file FROM resources WHERE resource_id=?"
            };
            let file: String = sqlx::query_scalar(statement)
                .bind(p.resource()?)
                .fetch_optional(&d.pool)
                .await?
                .context("resource not found")?;
            service.document(&d, &file).await?
        }
        _ => anyhow::bail!("unknown document kind"),
    };
    tokio::task::spawn_blocking(move || {
        let raw: Box<RawValue> = serde_json::from_slice(&bytes)?;
        let offset = p.offset()?;
        let (total, items) = values::expand(&raw, p.pointer.as_deref().unwrap_or(""), offset)?;
        source_page(&d, &p, total as i64, offset, items, "{}")
    })
    .await?
}

fn source_page(
    d: &Dataset,
    p: &ViewerQuery,
    total: i64,
    offset: usize,
    items: Vec<Value>,
    metadata: &str,
) -> Result<Value> {
    let envelope =
        json!({"resource_id":p.resource_id,"node_id":p.node_id,"node_metadata_json":metadata});
    let reserved = serde_json::to_vec(&envelope)?.len() + 2048;
    ensure!(
        reserved < super::super::PAGE_BYTES,
        "container metadata exceeds page limit; inspect the source document"
    );
    let mut result = page_with_budget(
        &d.generation_id,
        total,
        offset,
        items,
        super::super::PAGE_BYTES - reserved,
    )?;
    result["resource_id"] = json!(p.resource_id.as_deref().unwrap_or(""));
    result["node_id"] = json!(p.node_id);
    result["node_metadata_json"] = json!(metadata);
    ensure!(
        serde_json::to_vec(&result)?.len() <= super::super::PAGE_BYTES,
        "source metadata exceeds response size; inspect the source document"
    );
    Ok(result)
}

pub(super) fn property_entry(
    node: &Node,
    property: &str,
    fact: &Fact,
    aliases: Option<&Vec<String>>,
) -> Result<Value> {
    let mut item = values::entry(property, &fact.value)?;
    item["entry_kind"] = json!("property");
    item["node_id"] = json!(node.node_id);
    item["class_name"] = json!(node.class_name);
    item["view"] = json!(node.view);
    for key in [
        "native_type",
        "status",
        "origin",
        "native_unit",
        "unit_evidence",
    ] {
        item[key] = fact.metadata.get(key).cloned().unwrap_or(Value::Null);
    }
    let mut metadata = fact.metadata.clone();
    if let Some(aliases) = aliases {
        metadata.insert("organized_aliases".into(), json!(aliases));
        item["label"] = json!(format!("{} · {}", property, aliases.join(", ")));
    }
    let metadata = serde_json::to_string(&metadata)?;
    item["metadata_json"] = json!(if metadata.len() <= 32768 {
        metadata
    } else {
        json!({"expand_metadata":true}).to_string()
    });
    let links = fact.links()?;
    item["links"]=json!(links.iter().take(40).map(|(ordinal,id)|json!({"label":format!("{property}[{ordinal}]"),"node_id":id,"relationship":"property","ordinal":ordinal})).collect::<Vec<_>>());
    if links.len() > 40 {
        item["expanded"] = json!(false);
    }
    Ok(item)
}
