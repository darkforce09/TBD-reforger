//! Source facts retain their original JSON value tokens and all native metadata.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, value::RawValue};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub resource_id: String,
    pub root_node: String,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
pub struct Node {
    pub node_id: String,
    pub view: String,
    pub class_name: String,
    pub instance_name: String,
    pub native_instance_id: Option<String>,
    pub instance_identity_kind: String,
    pub resource_name: String,
    pub source_addons: Vec<String>,
    pub ancestor_id: Option<String>,
    #[serde(default)]
    pub ancestor_resource_name: Option<String>,
    #[serde(default)]
    pub selection: Option<String>,
    pub children: Vec<String>,
    pub declared_properties: Vec<String>,
    pub properties: BTreeMap<String, Fact>,
}

#[derive(Debug, Deserialize)]
pub struct Fact {
    pub value: Box<RawValue>,
    #[serde(flatten)]
    pub metadata: BTreeMap<String, Value>,
}

impl Fact {
    pub fn text(&self, key: &str) -> &str {
        self.metadata.get(key).and_then(Value::as_str).unwrap_or("")
    }
    pub fn links(&self) -> Result<Vec<(usize, String)>> {
        let mut links = Vec::new();
        match self.text("native_type") {
            "OBJECT" => {
                let value: Value = serde_json::from_str(self.value.get())?;
                if let Some(id) = value.get("node_id").and_then(Value::as_str) {
                    links.push((0, id.to_owned()));
                }
            }
            "OBJECT_ARRAY" => {
                let values: Vec<Value> = serde_json::from_str(self.value.get())?;
                for (ordinal, value) in values.iter().enumerate() {
                    if let Some(id) = value.get("node_id").and_then(Value::as_str) {
                        links.push((ordinal, id.to_owned()));
                    }
                }
            }
            _ => {}
        }
        Ok(links)
    }
}

impl Node {
    pub fn metadata(&self) -> Value {
        serde_json::json!({"node_id":self.node_id,"view":self.view,"class_name":self.class_name,
            "instance_name":self.instance_name,"native_instance_id":self.native_instance_id,
            "instance_identity_kind":self.instance_identity_kind,"resource_name":self.resource_name,
            "source_addons":self.source_addons,"ancestor_id":self.ancestor_id,
            "ancestor_resource_name":self.ancestor_resource_name,"selection":self.selection,
            "children":self.children,"declared_properties":self.declared_properties})
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Reference {
    pub node_id: String,
    pub property: String,
    pub resource_name: String,
    pub kind: String,
    pub method: String,
}

pub fn resolve_pointer(raw: &RawValue, pointer: &str) -> Result<Box<RawValue>> {
    ensure!(
        pointer.is_empty() || pointer.starts_with('/'),
        "invalid JSON pointer"
    );
    let mut current = RawValue::from_string(raw.get().to_owned())?;
    for part in pointer.split('/').skip(1) {
        let key = part.replace("~1", "/").replace("~0", "~");
        current = if current.get().trim_start().starts_with('[') {
            let values: Vec<Box<RawValue>> = serde_json::from_str(current.get())?;
            values
                .into_iter()
                .nth(key.parse::<usize>()?)
                .context("array entry not found")?
        } else {
            let mut values: BTreeMap<String, Box<RawValue>> = serde_json::from_str(current.get())?;
            values.remove(&key).context("property not found")?
        };
    }
    Ok(current)
}

pub fn pointer_segment(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}
