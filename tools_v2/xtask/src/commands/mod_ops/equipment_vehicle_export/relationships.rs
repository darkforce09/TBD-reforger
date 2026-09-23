use std::collections::BTreeSet;

use serde_json::Value;

use super::ValidationReport;
use super::validation::{array, text};

pub(super) fn validate_references(record: &Value, snapshot: &Value, report: &mut ValidationReport) {
    let resource = text(record, "resource_name");
    let mut actual: Vec<_> = array(&record["references"])
        .iter()
        .map(|link| {
            (
                text(link, "node_id"),
                text(link, "property"),
                text(link, "resource_name"),
                text(link, "kind"),
                text(link, "method"),
            )
        })
        .collect();
    let mut expected = Vec::new();
    for node in array(&snapshot["nodes"]) {
        let container_resource = text(node, "resource_name");
        if container_resource != resource && gameplay(container_resource) {
            expected.push((
                text(node, "node_id"),
                "resource_name",
                container_resource,
                "gameplay",
                "BaseContainer.GetResourceName",
            ));
        }
        let Some(properties) = node["properties"].as_object() else {
            continue;
        };
        let declared: BTreeSet<_> = array(&node["declared_properties"])
            .iter()
            .filter_map(Value::as_str)
            .collect();
        let inferred: BTreeSet<_> = properties
            .iter()
            .filter(|(_, fact)| fact["origin"] == "declared")
            .map(|(name, _)| name.as_str())
            .collect();
        if declared != inferred {
            report.errors.push(format!(
                "{resource}: declared property inventory differs from origin evidence"
            ));
        }
        for (property, fact) in properties {
            if !["present", "error"].contains(&text(fact, "status")) {
                report.errors.push(format!("{resource}: enumerated source property cannot be marked absent or unavailable: {property}"));
            }
            let values: Vec<_> = match text(fact, "native_type") {
                "RESOURCE_NAME" => fact["value"].as_str().into_iter().collect(),
                "RESOURCE_NAME_ARRAY" => array(&fact["value"])
                    .iter()
                    .filter_map(Value::as_str)
                    .collect(),
                _ => Vec::new(),
            };
            for target in values.into_iter().filter(|s| !s.is_empty()) {
                let kind = if gameplay(target) {
                    "gameplay"
                } else {
                    "binary"
                };
                expected.push((
                    text(node, "node_id"),
                    property.as_str(),
                    target,
                    kind,
                    "BaseContainer.Get",
                ));
            }
        }
    }
    actual.sort_unstable();
    expected.sort_unstable();
    if expected != actual {
        report.errors.push(format!(
            "{resource}: source resource relationships are omitted, misclassified or invented"
        ));
    }
}

pub(super) fn validate_types(
    record: &Value,
    snapshot: &Value,
    generation: &Value,
    report: &mut ValidationReport,
) {
    let resource = text(record, "resource_name");
    let mut expected = BTreeSet::new();
    for node in array(&snapshot["nodes"]) {
        expected.insert(text(node, "class_name"));
        if let Some(properties) = node["properties"].as_object() {
            for fact in properties.values() {
                if let Some(base) = fact["object_base_class"].as_str().filter(|s| !s.is_empty()) {
                    expected.insert(base);
                }
            }
        }
    }
    let actual: BTreeSet<_> = array(&record["type_names"])
        .iter()
        .filter_map(Value::as_str)
        .collect();
    if actual != expected {
        report
            .errors
            .push(format!("{resource}: native type inventory is incomplete"));
    }
    for name in actual {
        if generation["type_hierarchy"]["types"].get(name).is_none() {
            report.errors.push(format!(
                "{resource}: native type is missing from hierarchy: {name}"
            ));
        }
    }
}

pub(super) fn validate_hierarchy(generation: &Value, report: &mut ValidationReport) {
    let Some(types) = generation["type_hierarchy"]["types"].as_object() else {
        return;
    };
    for (name, detail) in types {
        if detail["status"] == "unavailable"
            && (detail["reason"].as_str().is_none_or(str::is_empty)
                || !array(&detail["ancestor_types"]).is_empty())
        {
            report.errors.push(format!("native type {name}: unavailable hierarchy must explain its limitation and cannot claim ancestors"));
        }
        if detail["status"] == "present" && !detail["reason"].is_null() {
            report.errors.push(format!(
                "native type {name}: present hierarchy has a failure reason"
            ));
        }
        for base in array(&detail["ancestor_types"])
            .iter()
            .filter_map(Value::as_str)
        {
            if base == name || !types.contains_key(base) {
                report
                    .errors
                    .push(format!("native type {name}: invalid ancestor {base}"));
            } else if array(&types[base]["ancestor_types"])
                .iter()
                .any(|v| v == name)
            {
                report
                    .errors
                    .push(format!("native type {name}: cyclic native inheritance"));
            }
        }
    }
}

fn gameplay(name: &str) -> bool {
    [".et", ".conf", ".gamemat", ".ragdoll"]
        .iter()
        .any(|extension| name.ends_with(extension))
}
