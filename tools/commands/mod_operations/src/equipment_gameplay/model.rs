//! The source and compact document shapes of an equipment gameplay generation.
//!
//! **Role:** the serde shapes of source facts, nodes and documents and of the compact facts, nodes,
//! resources and index entries.
//! **Position:** under [`crate::equipment_gameplay`]; the projection and the validation read and
//! write them.
//! **Signals & state:** none; plain data.
//! **Invariants:** a compact value keeps its raw JSON text, so a number is written back byte for
//! byte.

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
    pub(crate) node_id: String,
    pub class_name: String,
    pub properties: BTreeMap<String, SourceFact>,
    #[serde(flatten)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
pub(super) struct SourceDocument {
    pub(crate) resource_id: String,
    pub nodes: Vec<SourceNode>,
}

#[derive(Deserialize, Serialize, Clone)]
pub(super) struct CompactFact {
    pub(crate) definition_id: String,
    pub status: String,
    pub value: Box<RawValue>,
    pub origin: String,
    pub reason: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct CompactNode {
    pub(crate) node_id: String,
    pub class_name: String,
    pub properties: BTreeMap<String, CompactFact>,
    #[serde(flatten)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct CompactResource {
    pub document_type: String,
    pub schema_version: u32,
    pub(crate) resource_id: String,
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
    pub(crate) resource_id: String,
    pub resource_name: String,
    pub resource_file: String,
    pub domains: Vec<String>,
}

pub(super) fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
