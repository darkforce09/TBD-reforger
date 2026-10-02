use serde::{Deserialize, Serialize};
use serde_json::{Value, value::RawValue};
use std::collections::BTreeMap;

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct SourceFact {
    pub value: Box<RawValue>,
    #[serde(flatten)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct SourceNode {
    pub node_id: String,
    pub class_name: String,
    pub properties: BTreeMap<String, SourceFact>,
    #[serde(flatten)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
pub(super) struct SourceDocument {
    pub resource_id: String,
    pub nodes: Vec<SourceNode>,
}

#[derive(Deserialize, Serialize, Clone)]
pub(super) struct CompactFact {
    pub definition_id: String,
    pub status: String,
    pub value: Box<RawValue>,
    pub origin: String,
    pub reason: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct CompactNode {
    pub node_id: String,
    pub class_name: String,
    pub properties: BTreeMap<String, CompactFact>,
    #[serde(flatten)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct CompactResource {
    pub document_type: String,
    pub schema_version: u32,
    pub resource_id: String,
    pub resource_name: String,
    pub resource_guid: Option<String>,
    pub identity_kind: String,
    pub source_addons: Vec<String>,
    pub root_node: String,
    pub nodes: Vec<CompactNode>,
    pub capabilities: BTreeMap<String, Vec<String>>,
    pub names: Vec<Value>,
    pub references: Vec<Value>,
    pub parent_prefab: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub(super) struct ResourceEntry {
    pub resource_id: String,
    pub resource_name: String,
    pub resource_file: String,
    pub domains: Vec<String>,
}

pub(super) fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
