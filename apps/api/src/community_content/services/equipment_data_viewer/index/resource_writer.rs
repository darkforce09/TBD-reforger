//! Index individual native instances and every field occurrence without copying values.
use super::super::source::document::{Reference, Snapshot};
use super::writer::BuildState;
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use sqlx::{QueryBuilder, Sqlite, SqliteConnection};
use std::collections::HashMap;

pub(super) async fn write(
    db: &mut SqliteConnection,
    resource: i64,
    source: &Snapshot,
    record: &Value,
    state: &mut BuildState,
) -> Result<()> {
    let mut nodes = HashMap::new();
    for node in &source.nodes {
        ensure!(
            nodes
                .insert(node.node_id.clone(), state.next_node)
                .is_none(),
            "duplicate source node"
        );
        state.next_node += 1;
    }
    ensure!(nodes.contains_key(&source.root_node), "missing root node");
    for chunk in source
        .nodes
        .iter()
        .enumerate()
        .collect::<Vec<_>>()
        .chunks(200)
    {
        let mut query = QueryBuilder::<Sqlite>::new("INSERT INTO nodes ");
        query.push_values(chunk, |mut row, (ordinal, n)| {
            row.push_bind(nodes[&n.node_id])
                .push_bind(resource)
                .push_bind(&n.node_id)
                .push_bind(if n.node_id == source.root_node {
                    -1
                } else {
                    *ordinal as i64
                })
                .push_bind(&n.class_name)
                .push_bind(&n.instance_name)
                .push_bind(&n.view)
                .push_bind(&n.native_instance_id)
                .push_bind(&n.instance_identity_kind);
        });
        query.build().execute(&mut *db).await?;
    }
    let mut occurrences = Vec::new();
    let mut edges = Vec::new();
    for node in &source.nodes {
        let id = nodes[&node.node_id];
        for (ordinal, child) in node.children.iter().enumerate() {
            edges.push((
                id,
                *nodes.get(child).context("missing child node")?,
                "child".to_owned(),
                "children".to_owned(),
                ordinal as i64,
            ));
        }
        if let Some(parent) = &node.ancestor_id {
            edges.push((
                id,
                *nodes.get(parent).context("missing ancestor")?,
                "ancestor".to_owned(),
                "ancestor_id".to_owned(),
                0,
            ));
        }
        for (property, fact) in &node.properties {
            ensure!(
                fact.text("status") != "error",
                "source fact contains an extraction error"
            );
            let next = state.fields.len() as i64 + 1;
            let field = *state
                .fields
                .entry((
                    node.class_name.clone(),
                    property.clone(),
                    fact.text("native_type").to_owned(),
                ))
                .or_insert(next);
            occurrences.push((
                id,
                field,
                fact.text("status"),
                fact.text("origin"),
                fact.metadata.get("native_unit").and_then(Value::as_str),
            ));
            for (ordinal, target) in fact.links()? {
                edges.push((
                    id,
                    *nodes.get(&target).context("missing object node")?,
                    "property".to_owned(),
                    property.clone(),
                    ordinal as i64,
                ));
            }
            state.fact_count += 1;
        }
    }
    for chunk in occurrences.chunks(500) {
        let mut q = QueryBuilder::<Sqlite>::new("INSERT INTO occurrences ");
        q.push_values(chunk, |mut row, a| {
            row.push_bind(a.0)
                .push_bind(a.1)
                .push_bind(a.2)
                .push_bind(a.3)
                .push_bind(a.4);
        });
        q.build().execute(&mut *db).await?;
    }
    for chunk in edges.chunks(500) {
        let mut q = QueryBuilder::<Sqlite>::new("INSERT INTO edges ");
        q.push_values(chunk, |mut row, a| {
            row.push_bind(a.0)
                .push_bind(a.1)
                .push_bind(&a.2)
                .push_bind(&a.3)
                .push_bind(a.4);
        });
        q.build().execute(&mut *db).await?;
    }
    for (capability, instances) in record["capabilities"].as_object().context("capabilities")? {
        for instance in instances.as_array().context("capability instances")? {
            let node_id = instance["node_id"].as_str().context("capability node")?;
            let native = source
                .nodes
                .iter()
                .find(|n| n.node_id == node_id)
                .context("capability source node missing")?;
            ensure!(native.view == "effective", "capability points to ancestor");
            sqlx::query("INSERT INTO capabilities VALUES (?,?,?)")
                .bind(resource)
                .bind(nodes[node_id])
                .bind(capability)
                .execute(&mut *db)
                .await?;
            for (alias, fact) in instance["facts"].as_object().context("capability facts")? {
                let property = fact["source"]["property"]
                    .as_str()
                    .context("fact source property")?;
                let native_fact = native
                    .properties
                    .get(property)
                    .context("organized fact missing from source")?;
                let field = state.fields[&(
                    native.class_name.clone(),
                    property.to_owned(),
                    native_fact.text("native_type").to_owned(),
                )];
                state
                    .aliases
                    .insert((field, capability.clone(), alias.clone()));
            }
        }
    }
    let references: Vec<Reference> = serde_json::from_value(record["references"].clone())?;
    for chunk in references.chunks(400) {
        for r in chunk {
            ensure!(
                nodes.contains_key(&r.node_id),
                "reference source node missing"
            );
        }
        let mut q = QueryBuilder::<Sqlite>::new(
            "INSERT INTO resource_references (resource,node,property,target,target_resource_name,kind,method) ",
        );
        q.push_values(chunk, |mut row, r| {
            row.push_bind(resource)
                .push_bind(nodes[&r.node_id])
                .push_bind(&r.property)
                .push_bind(state.resources.get(&r.resource_name).copied())
                .push_bind(&r.resource_name)
                .push_bind(&r.kind)
                .push_bind(&r.method);
        });
        q.build().execute(&mut *db).await?;
    }
    Ok(())
}
