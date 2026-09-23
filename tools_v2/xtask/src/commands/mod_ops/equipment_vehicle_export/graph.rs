use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::ValidationReport;
use super::relationships;
use super::validation::{array, text};

pub(super) fn validate(record: &Value, snapshot: &Value, report: &mut ValidationReport) {
    let resource = text(record, "resource_name");
    let mut nodes = BTreeMap::new();
    for node in array(&snapshot["nodes"]) {
        let id = text(node, "node_id");
        if nodes.insert(id, node).is_some() {
            report
                .errors
                .push(format!("{resource}: duplicate source node {id}"));
        }
    }
    report.source_node_count += nodes.len();
    if !nodes.contains_key(text(snapshot, "root_node")) {
        report
            .errors
            .push(format!("{resource}: missing source root"));
    }
    let mut edges = BTreeMap::new();
    for (id, node) in &nodes {
        let mut links = Vec::new();
        for child in array(&node["children"]).iter().filter_map(Value::as_str) {
            links.push(child.to_owned());
        }
        if let Some(properties) = node["properties"].as_object() {
            for (property, fact) in properties {
                report.fact_count += 1;
                if fact["source"]["resource_name"] != resource
                    || fact["source"]["node_id"] != *id
                    || fact["source"]["property"] != *property
                {
                    report.errors.push(format!(
                        "{resource} {id}/{property}: incorrect source location"
                    ));
                }
                validate_fact(fact, &format!("{resource} {id}/{property}"), report);
                collect_links(&fact["value"], &mut links);
            }
            for property in array(&node["declared_properties"])
                .iter()
                .filter_map(Value::as_str)
            {
                if properties
                    .get(property)
                    .is_none_or(|fact| fact["origin"] != "declared")
                {
                    report.errors.push(format!(
                        "{resource} {id}: invalid declared property {property}"
                    ));
                }
            }
        }
        for link in &links {
            if !nodes.contains_key(link.as_str()) {
                report.errors.push(format!(
                    "{resource} {id}: missing linked source node {link}"
                ));
            } else if nodes[link.as_str()]["view"] != node["view"] {
                report.errors.push(format!(
                    "{resource} {id}: object relationship crosses effective/ancestor views: {link}"
                ));
            }
        }
        if let Some(ancestor) = node["ancestor_id"].as_str() {
            if nodes.get(ancestor).is_none_or(|n| n["view"] != "ancestor") {
                report
                    .errors
                    .push(format!("{resource} {id}: invalid ancestor relationship"));
            }
            links.push(ancestor.to_owned());
        }
        let native_id = node["native_instance_id"].as_str();
        let native_identity = node["instance_identity_kind"] == "native_container_id";
        if native_identity != native_id.is_some()
            || native_id.is_some_and(|v| {
                v != text(node, "resource_name")
                    || v.len() != 18
                    || !v.starts_with('{')
                    || !v.ends_with('}')
                    || !v.as_bytes()[1..17].iter().all(|b| b.is_ascii_hexdigit())
            })
        {
            report
                .errors
                .push(format!("{resource} {id}: invalid native instance identity"));
        }
        edges.insert((*id).to_owned(), links);
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    if has_cycle(
        text(snapshot, "root_node"),
        &edges,
        &mut visiting,
        &mut visited,
    ) {
        report
            .errors
            .push(format!("{resource}: cyclic source graph"));
    }
    if visited.len() != nodes.len() {
        report
            .errors
            .push(format!("{resource}: disconnected source nodes"));
    }
    if let Some(capabilities) = record["capabilities"].as_object() {
        for (capability, installations) in capabilities {
            let mut seen = BTreeSet::new();
            for installation in array(installations) {
                let id = text(installation, "node_id");
                if !seen.insert(id) {
                    report.errors.push(format!(
                        "{resource}: duplicate {capability} installation {id}"
                    ));
                }
                if nodes.get(id).is_none_or(|node| node["view"] != "effective") {
                    report.errors.push(format!(
                        "{resource}: {capability} points outside the effective graph: {id}"
                    ));
                }
                if let Some(facts) = installation["facts"].as_object() {
                    let native_count = nodes
                        .get(id)
                        .and_then(|n| n["properties"].as_object())
                        .map_or(0, |p| p.len());
                    let sources: BTreeSet<_> = facts
                        .values()
                        .map(|f| text(&f["source"], "property"))
                        .collect();
                    if native_count != facts.len() || sources.len() != facts.len() {
                        report.errors.push(format!("{resource}: organized capability omits or duplicates native properties: {id}"));
                    }
                    for (field, fact) in facts {
                        if fact["source"]["node_id"] != id {
                            report.errors.push(format!("{resource}: organized fact belongs to a different installation: {id}"));
                        }
                        if !snake_case(field) {
                            report
                                .errors
                                .push(format!("{resource}: nonstandard organized field {field}"));
                        }
                        match_source(
                            fact,
                            &nodes,
                            &format!("{resource} {capability}/{field}"),
                            report,
                        );
                    }
                }
            }
        }
    }
    for name in array(&record["names"]) {
        match_source(&name["source_text"], &nodes, resource, report);
        let english = &name["display_name_en"];
        validate_fact(english, resource, report);
        if english["source"]["method"] != "WidgetManager.Translate"
            || english["source"]["node_id"] != name["node_id"]
        {
            report
                .errors
                .push(format!("{resource}: untraceable English name"));
        }
        if english["status"] == "present"
            && english["value"]
                .as_str()
                .is_none_or(|s| s.is_empty() || s.starts_with('#'))
        {
            report.errors.push(format!(
                "{resource}: unresolved English name marked present"
            ));
        }
    }
    for reference in array(&record["references"]) {
        let node = nodes.get(text(reference, "node_id"));
        let property = text(reference, "property");
        let target = &reference["resource_name"];
        if let Some(node) = node {
            let fact = &node["properties"][property];
            let backed = if reference["method"] == "BaseContainer.GetResourceName" {
                property == "resource_name" && node["resource_name"] == *target
            } else {
                fact["value"] == *target || array(&fact["value"]).contains(target)
            };
            if !backed {
                report.errors.push(format!(
                    "{resource}: reference not backed by source: {target}"
                ));
            }
        } else {
            report
                .errors
                .push(format!("{resource}: reference has missing source node"));
        }
    }
    relationships::validate_references(record, snapshot, report);
    validate_identity(record, report);
}

fn validate_identity(record: &Value, report: &mut ValidationReport) {
    let name = text(record, "resource_name");
    let id = text(record, "resource_id");
    let guid = record["resource_guid"].as_str();
    let expected = if record["identity_kind"] == "resource_guid" {
        if guid.is_none_or(|g| {
            g.len() != 16
                || !g.bytes().all(|b| b.is_ascii_hexdigit())
                || !name.starts_with(&format!("{{{g}}}"))
        }) {
            report
                .errors
                .push(format!("{name}: invalid native GUID identity"));
        }
        format!("guid:{}", guid.unwrap_or_default())
    } else {
        if guid.is_some() {
            report
                .errors
                .push(format!("{name}: resource-name identity has a GUID"));
        }
        format!("resource:{name}")
    };
    if id != expected {
        report
            .errors
            .push(format!("{name}: identity does not match its source"));
    }
}

fn match_source(
    fact: &Value,
    nodes: &BTreeMap<&str, &Value>,
    context: &str,
    report: &mut ValidationReport,
) {
    validate_fact(fact, context, report);
    let source = &fact["source"];
    let node = nodes.get(text(source, "node_id"));
    let native = node.map(|n| &n["properties"][text(source, "property")]);
    if fact["status"] == "present" && native != Some(fact) {
        report.errors.push(format!(
            "{context}: organized fact differs from native source fact"
        ));
    }
    if ["not_present", "not_applicable", "unavailable"].contains(&text(fact, "status"))
        && native.is_some_and(|n| !n.is_null())
    {
        report
            .errors
            .push(format!("{context}: existing property marked absent"));
    }
}

fn validate_fact(fact: &Value, context: &str, report: &mut ValidationReport) {
    if fact["status"] == "error" {
        report.errors.push(format!("{context}: {}", fact["reason"]));
    }
    if fact["status"] == "unavailable" {
        report
            .warnings
            .push(format!("{context}: {}", fact["reason"]));
    }
    if fact["status"] != "present" {
        return;
    }
    let value = &fact["value"];
    let native = text(fact, "native_type");
    let valid = match native {
        "INTEGER" | "FLAGS" => value.as_i64().is_some(),
        "SCALAR" => value.is_number(),
        "BOOLEAN" => value.is_boolean(),
        "STRING" | "RESOURCE_NAME" | "TEXTURE" => value.is_string(),
        "VECTOR2" => numeric_tuple(value, 2),
        "VECTOR3" => numeric_tuple(value, 3),
        "VECTOR4" | "COLOR" => numeric_tuple(value, 4),
        "OBJECT" => value.is_null() || value.get("node_id").is_some_and(Value::is_string),
        "OBJECT_ARRAY" => value.as_array().is_some_and(|a| {
            a.iter()
                .all(|v| v.is_null() || v.get("node_id").is_some_and(Value::is_string))
        }),
        "SCALAR_ARRAY" => value
            .as_array()
            .is_some_and(|a| a.iter().all(Value::is_number)),
        "INTEGER_ARRAY" => value
            .as_array()
            .is_some_and(|a| a.iter().all(|v| v.as_i64().is_some())),
        "BOOLEAN_ARRAY" => value
            .as_array()
            .is_some_and(|a| a.iter().all(Value::is_boolean)),
        "STRING_ARRAY" | "RESOURCE_NAME_ARRAY" => value
            .as_array()
            .is_some_and(|a| a.iter().all(Value::is_string)),
        "VECTOR3_ARRAY" => value
            .as_array()
            .is_some_and(|a| a.iter().all(|v| numeric_tuple(v, 3))),
        "VECTOR2_ARRAY" => value
            .as_array()
            .is_some_and(|a| a.iter().all(|v| numeric_tuple(v, 2))),
        _ => false,
    };
    if !valid {
        report.errors.push(format!(
            "{context}: value does not preserve native type {native}"
        ));
    }
}

fn numeric_tuple(value: &Value, len: usize) -> bool {
    value
        .as_array()
        .is_some_and(|a| a.len() == len && a.iter().all(Value::is_number))
}
fn snake_case(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
fn collect_links(value: &Value, out: &mut Vec<String>) {
    if let Some(id) = value.get("node_id").and_then(Value::as_str) {
        out.push(id.to_owned());
    }
    if let Some(values) = value.as_array() {
        for value in values {
            collect_links(value, out);
        }
    }
}
fn has_cycle(
    node: &str,
    edges: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> bool {
    if visiting.len() > 128 || visiting.contains(node) {
        return true;
    }
    if !visited.insert(node.to_owned()) {
        return false;
    }
    visiting.insert(node.to_owned());
    let cycle = edges.get(node).is_some_and(|links| {
        links
            .iter()
            .any(|link| has_cycle(link, edges, visiting, visited))
    });
    visiting.remove(node);
    cycle
}
