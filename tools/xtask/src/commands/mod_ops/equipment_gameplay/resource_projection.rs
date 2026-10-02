use super::{model::*, policy::Policy};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct ProjectionState {
    pub definitions: BTreeMap<String, Value>,
    definition_keys: BTreeMap<String, String>,
    pub decisions: BTreeMap<String, u64>,
    pub sections: BTreeMap<String, u64>,
    pub nodes: u64,
    pub facts: u64,
}

impl ProjectionState {
    fn definition(
        &mut self,
        class: &str,
        property: &str,
        fact: &SourceFact,
        rule: &super::policy::Rule,
    ) -> Result<String> {
        let source = fact.metadata.get("source").context("missing fact source")?;
        let mut definition = json!({"class_name":class,"property":property,"field_name":field_name(property),"section":rule.section,"disposition":rule.disposition,"follow_reference":rule.follow_reference,"method":source["method"]});
        for key in [
            "native_type",
            "native_unit",
            "unit_evidence",
            "object_base_class",
            "enum_values",
        ] {
            definition[key] = fact.metadata.get(key).cloned().unwrap_or(Value::Null);
        }
        let key = definition.to_string();
        if let Some(id) = self.definition_keys.get(&key) {
            return Ok(id.clone());
        }
        let id = format!("field_{}", self.definitions.len());
        self.definition_keys.insert(key, id.clone());
        self.definitions.insert(id.clone(), definition);
        Ok(id)
    }
}

pub(super) fn field_name(property: &str) -> String {
    use heck::ToSnakeCase;
    if property == "AmmoTemplate" {
        return "default_projectile".into();
    }
    if property == "Trigger Offset" {
        return "trigger_offset_vector3".into();
    }
    property.to_snake_case()
}

pub(super) fn project(
    policy: &Policy,
    source: SourceDocument,
    record: Value,
    state: &mut ProjectionState,
) -> Result<CompactResource> {
    let all: BTreeMap<_, _> = source
        .nodes
        .iter()
        .map(|n| (n.node_id.as_str(), n))
        .collect();
    let root = all.get("root").context("missing source root")?;
    let parent_prefab = root
        .metadata
        .get("ancestor_id")
        .and_then(Value::as_str)
        .and_then(|id| all.get(id))
        .and_then(|n| n.metadata.get("resource_name"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let mut walk = Walk {
        policy,
        all: &all,
        state,
        nodes: BTreeMap::new(),
        visiting: BTreeSet::new(),
        capabilities: BTreeMap::new(),
        references: Vec::new(),
        resource_name: text(&record["resource_name"]),
    };
    walk.visit("root", true)?;
    let selected: BTreeSet<_> = walk.nodes.keys().cloned().collect();
    let mut names = Vec::new();
    for name in record["names"].as_array().context("source names")? {
        let node_id = text(&name["node_id"]);
        if selected.contains(node_id) && walk.nodes[node_id].properties.contains_key("Name") {
            names.push(json!({"node_id":node_id,"source_property":"Name","display_name_en":{
                "value":name["display_name_en"]["value"],"status":name["display_name_en"]["status"],
                "reason":name["display_name_en"]["reason"],"method":name["display_name_en"]["source"]["method"]},"locale":name["locale"]}));
        }
    }
    // Keep the native traversal order, independent of opaque identifier spelling.
    let nodes = source
        .nodes
        .iter()
        .filter_map(|n| walk.nodes.remove(&n.node_id))
        .collect();
    Ok(CompactResource {
        document_type: "gameplay_resource".into(),
        schema_version: 1,
        resource_id: source.resource_id,
        resource_name: text(&record["resource_name"]).into(),
        resource_guid: record["resource_guid"].as_str().map(str::to_owned),
        identity_kind: text(&record["identity_kind"]).into(),
        source_addons: serde_json::from_value(record["source_addons"].clone())?,
        root_node: "root".into(),
        nodes,
        capabilities: walk.capabilities,
        names,
        references: walk.references,
        parent_prefab,
    })
}

struct Walk<'a> {
    policy: &'a Policy,
    all: &'a BTreeMap<&'a str, &'a SourceNode>,
    state: &'a mut ProjectionState,
    nodes: BTreeMap<String, CompactNode>,
    visiting: BTreeSet<String>,
    capabilities: BTreeMap<String, Vec<String>>,
    references: Vec<Value>,
    resource_name: &'a str,
}

impl Walk<'_> {
    fn visit(&mut self, id: &str, required: bool) -> Result<bool> {
        if self.nodes.contains_key(id) {
            return Ok(true);
        }
        ensure!(self.visiting.insert(id.into()), "container cycle at {id}");
        let node = *self
            .all
            .get(id)
            .context("dangling native object reference")?;
        ensure!(
            node.metadata.get("view").and_then(Value::as_str) == Some("effective"),
            "effective relationship targets ancestor"
        );
        let selected = self.policy.selected(&node.class_name)?;
        if !selected && !required {
            self.visiting.remove(id);
            return Ok(false);
        }
        let mut metadata = node.metadata.clone();
        let ancestor = metadata.remove("ancestor_id");
        let ancestor_resource = ancestor
            .as_ref()
            .and_then(Value::as_str)
            .and_then(|v| self.all.get(v))
            .and_then(|n| n.metadata.get("resource_name"))
            .cloned()
            .unwrap_or(Value::Null);
        metadata.insert("ancestor_resource_name".into(), ancestor_resource);
        metadata.insert("ancestor_id".into(), Value::Null);
        metadata.insert(
            "selection".into(),
            json!(if selected {
                "retained"
            } else {
                "identity_only"
            }),
        );
        metadata.remove("declared_properties");
        let mut properties = BTreeMap::new();
        let mut children = Vec::<String>::new();
        let mut sections = BTreeSet::new();
        if selected {
            for (property, fact) in &node.properties {
                let native_type = fact
                    .metadata
                    .get("native_type")
                    .and_then(Value::as_str)
                    .context("native type")?;
                let rule = self.policy.rule(&node.class_name, property, native_type)?;
                *self
                    .state
                    .decisions
                    .entry(format!(
                        "{}\t{}\t{}",
                        node.class_name, property, native_type
                    ))
                    .or_default() += 1;
                if rule.disposition == "exclude" {
                    continue;
                }
                ensure!(
                    fact.metadata.get("status").and_then(Value::as_str) != Some("error"),
                    "retained source read failed"
                );
                let source = fact.metadata.get("source").context("source evidence")?;
                ensure!(
                    source["resource_name"] == self.resource_name
                        && source["node_id"] == id
                        && source["property"] == *property,
                    "fact source context differs from owning resource/container"
                );
                let value: Value = serde_json::from_str(fact.value.get())?;
                if matches!(native_type, "OBJECT" | "OBJECT_ARRAY") {
                    for (_, target) in object_links(&value) {
                        if self.visit(&target, rule.disposition != "traverse_required_container")?
                            && rule.disposition == "traverse_required_container"
                        {
                            children.push(target);
                        }
                    }
                }
                if rule.disposition == "traverse_required_container" {
                    continue;
                }
                if matches!(native_type, "RESOURCE_NAME" | "RESOURCE_NAME_ARRAY") {
                    let names: Vec<_> = if native_type == "RESOURCE_NAME" {
                        vec![value.clone()]
                    } else {
                        value.as_array().context("resource array")?.clone()
                    };
                    for (ordinal, name) in names.iter().enumerate() {
                        let name = name.as_str().context("resource reference")?;
                        if name.is_empty() {
                            continue;
                        }
                        let gameplay = rule.follow_reference && gameplay_resource(name);
                        self.references.push(json!({"node_id":id,"property":property,"ordinal":ordinal,"resource_name":name,"kind":if gameplay {"gameplay"} else {"binary"},"method":source["method"]}));
                    }
                }
                let definition_id =
                    self.state
                        .definition(&node.class_name, property, fact, rule)?;
                sections.insert(rule.section.clone());
                *self.state.sections.entry(rule.section.clone()).or_default() += 1;
                properties.insert(
                    property.clone(),
                    CompactFact {
                        definition_id,
                        value: fact.value.clone(),
                        status: text(&fact.metadata["status"]).into(),
                        origin: text(&fact.metadata["origin"]).into(),
                        reason: fact
                            .metadata
                            .get("reason")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    },
                );
                self.state.facts += 1;
            }
            for child in node
                .metadata
                .get("children")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let child = child.as_str().context("child identity")?;
                if self.visit(child, false)? && !children.iter().any(|v| v == child) {
                    children.push(child.into());
                }
            }
        }
        for section in sections {
            self.capabilities
                .entry(section)
                .or_default()
                .push(id.into());
        }
        metadata.insert("children".into(), json!(children));
        metadata.insert(
            "declared_properties".into(),
            json!(
                properties
                    .iter()
                    .filter(|(_, f)| f.origin == "declared")
                    .map(|(p, _)| p)
                    .collect::<Vec<_>>()
            ),
        );
        self.nodes.insert(
            id.into(),
            CompactNode {
                node_id: id.into(),
                class_name: node.class_name.clone(),
                properties,
                metadata,
            },
        );
        self.visiting.remove(id);
        self.state.nodes += 1;
        Ok(true)
    }
}

pub(super) fn object_links(value: &Value) -> Vec<(usize, String)> {
    if let Some(id) = value.get("node_id").and_then(Value::as_str) {
        return vec![(0, id.into())];
    }
    value
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(i, v)| {
            v.get("node_id")
                .and_then(Value::as_str)
                .map(|s| (i, s.into()))
        })
        .collect()
}

pub(super) fn gameplay_resource(name: &str) -> bool {
    [".et", ".conf", ".gamemat"]
        .iter()
        .any(|ext| name.ends_with(ext))
}
