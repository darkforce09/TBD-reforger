//! One gameplay resource's check: location, identity, fields and references.
//!
//! **Role:** inspects one gameplay resource document and records its findings.
//! **Position:** under [`crate::equipment_gameplay`]; `validation.rs` calls it per resource.
//! **Signals & state:** none; findings go into the caller's validation report.
//! **Invariants:** a resource outside its expected folder or with an unknown field is a finding.

use super::super::equipment_vehicle_export::ValidationReport;
use super::{
    model::*,
    policy::Policy,
    resource_projection::{gameplay_resource, object_links},
};
use crate::error::{Result, ResultExt, ensure};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn inspect(
    resource: &CompactResource,
    definitions: &Map<String, Value>,
    types: &Value,
    policy: &Policy,
    report: &mut ValidationReport,
) -> Result<()> {
    let nodes: BTreeMap<_, _> = resource
        .nodes
        .iter()
        .map(|n| (n.node_id.as_str(), n))
        .collect();
    ensure!(
        nodes.len() == resource.nodes.len(),
        "duplicate installation identity"
    );
    ensure!(
        nodes.contains_key(resource.root_node.as_str()),
        "missing root container"
    );
    let mut sections: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut expected_references = Vec::new();
    let mut edges: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in &resource.nodes {
        let selected = policy.selected(&node.class_name)?;
        ensure!(
            selected || node.properties.is_empty(),
            "excluded configuration has values"
        );
        ensure!(
            node.metadata["selection"]
                == if selected {
                    "retained"
                } else {
                    "identity_only"
                },
            "incorrect node selection"
        );
        let children: Vec<String> = serde_json::from_value(node.metadata["children"].clone())?;
        edges
            .entry(node.node_id.clone())
            .or_default()
            .extend(children);
        for (property, fact) in &node.properties {
            let definition = definitions
                .get(&fact.definition_id)
                .context("missing field definition")?;
            ensure!(
                definition["class_name"] == node.class_name && definition["property"] == *property,
                "field definition ownership mismatch"
            );
            let native_type = text(&definition["native_type"]);
            let rule = policy.rule(&node.class_name, property, native_type)?;
            ensure!(
                matches!(
                    rule.disposition.as_str(),
                    "retain_value" | "retain_relationship"
                ),
                "excluded field emitted: {property}"
            );
            ensure!(
                definition["disposition"] == rule.disposition
                    && definition["section"] == rule.section
                    && definition["follow_reference"] == rule.follow_reference,
                "field policy mismatch"
            );
            ensure!(fact.status != "error", "source read failed: {property}");
            ensure!(
                !text(&definition["method"]).is_empty(),
                "missing extraction method"
            );
            if fact.status == "unavailable" {
                ensure!(
                    fact.reason.as_ref().is_some_and(|s| !s.is_empty()),
                    "unavailable without reason"
                );
            }
            let value: Value = serde_json::from_str(fact.value.get())?;
            if native_type == "OBJECT" || native_type == "OBJECT_ARRAY" {
                edges
                    .entry(node.node_id.clone())
                    .or_default()
                    .extend(object_links(&value).into_iter().map(|(_, id)| id));
            }
            if matches!(native_type, "TYPENAME" | "TYPENAME_ARRAY") {
                let values = if native_type == "TYPENAME" {
                    vec![value.clone()]
                } else {
                    value.as_array().context("native type array")?.clone()
                };
                for typename in values
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    ensure!(
                        types["types"].get(typename).is_some(),
                        "native type hierarchy missing: {typename}"
                    );
                }
            }
            if matches!(native_type, "RESOURCE_NAME" | "RESOURCE_NAME_ARRAY") {
                let values = if native_type == "RESOURCE_NAME" {
                    vec![value.clone()]
                } else {
                    value.as_array().context("resource array")?.clone()
                };
                for name in values
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    let kind = if rule.follow_reference && gameplay_resource(name) {
                        "gameplay"
                    } else {
                        "binary"
                    };
                    expected_references.push((
                        node.node_id.clone(),
                        property.clone(),
                        name.to_owned(),
                        kind.to_owned(),
                    ));
                }
            }
            sections
                .entry(rule.section.clone())
                .or_default()
                .insert(node.node_id.clone());
            report.fact_count += 1;
        }
        report.source_node_count += 1;
    }
    let mut actual_references: Vec<_> = resource
        .references
        .iter()
        .map(|r| {
            (
                text(&r["node_id"]).to_owned(),
                text(&r["property"]).to_owned(),
                text(&r["resource_name"]).to_owned(),
                text(&r["kind"]).to_owned(),
            )
        })
        .collect();
    actual_references.sort();
    expected_references.sort();
    ensure!(
        actual_references == expected_references,
        "resource relationships disagree with retained facts"
    );
    let actual_sections: BTreeMap<_, BTreeSet<_>> = resource
        .capabilities
        .iter()
        .map(|(s, ids)| (s.clone(), ids.iter().cloned().collect()))
        .collect();
    ensure!(
        actual_sections == sections,
        "organized sections disagree with retained facts"
    );
    let mut visited = BTreeSet::new();
    walk(
        &resource.root_node,
        &edges,
        &nodes,
        &mut BTreeSet::new(),
        &mut visited,
    )?;
    ensure!(
        visited.len() == nodes.len(),
        "unreachable retained containers"
    );
    Ok(())
}

fn walk(
    id: &str,
    edges: &BTreeMap<String, Vec<String>>,
    nodes: &BTreeMap<&str, &CompactNode>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> Result<()> {
    ensure!(
        nodes.contains_key(id),
        "dangling container relationship: {id}"
    );
    if visited.contains(id) {
        return Ok(());
    }
    ensure!(visiting.insert(id.into()), "container cycle: {id}");
    ensure!(visiting.len() <= 256, "container traversal limit");
    for target in edges.get(id).into_iter().flatten() {
        walk(target, edges, nodes, visiting, visited)?;
    }
    visiting.remove(id);
    visited.insert(id.into());
    Ok(())
}
