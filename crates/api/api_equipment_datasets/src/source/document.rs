//! Source facts retain their original JSON value tokens and all native metadata.
use crate::error::{Required, Result, ensure};
use api_identifiers::{EquipmentNativeInstanceId, EquipmentNodeId, EquipmentResourceId};
use serde::{Deserialize, Serialize};
use serde_json::{Value, value::RawValue};
use std::collections::BTreeMap;

/// One exported resource document: its node tree with every property fact.
#[derive(Debug, Deserialize)]
pub struct Snapshot {
    /// The document format: `2` is self-describing, `1` is a gameplay resource whose facts take
    /// their metadata from the shared field definitions.
    pub schema_version: u32,
    /// The resource the document describes.
    pub resource_id: EquipmentResourceId,
    /// The node the resource's tree starts at.
    pub root_node: EquipmentNodeId,
    /// Every node of the tree, in export order.
    pub nodes: Vec<Node>,
}

/// One object of a resource's tree, with the native metadata the export recorded for it.
#[derive(Debug, Deserialize)]
pub struct Node {
    /// The node's identity within its generation.
    pub node_id: EquipmentNodeId,
    /// The view the node belongs to (`effective` for the resolved tree).
    pub view: String,
    /// The native class of the object.
    pub class_name: String,
    /// The object's instance name; empty when unnamed.
    pub instance_name: String,
    /// The engine's own instance identity, when the export could read one.
    pub native_instance_id: Option<EquipmentNativeInstanceId>,
    /// How the node's identity was derived.
    pub instance_identity_kind: String,
    /// The resource file the object is declared in.
    pub resource_name: String,
    /// The addons that contribute to the object.
    pub source_addons: Vec<String>,
    /// The node this one inherits from, when it has one.
    pub ancestor_id: Option<EquipmentNodeId>,
    /// The resource the ancestor is declared in.
    #[serde(default)]
    pub ancestor_resource_name: Option<String>,
    /// The selection the object belongs to, when the export recorded one.
    #[serde(default)]
    pub selection: Option<String>,
    /// The child nodes, in declaration order.
    pub children: Vec<EquipmentNodeId>,
    /// The properties the object declares itself, rather than inherits.
    pub declared_properties: Vec<String>,
    /// Every property fact of the object, keyed by property name.
    pub properties: BTreeMap<String, Fact>,
}

/// One property value with its native metadata; the value keeps its original JSON tokens.
#[derive(Debug, Deserialize)]
pub struct Fact {
    /// The value exactly as exported.
    pub value: Box<RawValue>,
    /// Every other key of the fact: the native type, the source, the field definition.
    #[serde(flatten)]
    pub metadata: BTreeMap<String, Value>,
}

impl Fact {
    /// The metadata string `key`, or `""` when it is absent or not a string.
    pub fn text(&self, key: &str) -> &str {
        self.metadata.get(key).and_then(Value::as_str).unwrap_or("")
    }
    /// The nodes an `OBJECT` or `OBJECT_ARRAY` value points to, with their position in the value;
    /// empty for any other native type.
    pub fn links(&self) -> Result<Vec<(usize, EquipmentNodeId)>> {
        let mut links = Vec::new();
        match self.text("native_type") {
            "OBJECT" => {
                let value: Value = serde_json::from_str(self.value.get())?;
                if let Some(id) = value.get("node_id").and_then(Value::as_str) {
                    links.push((0, EquipmentNodeId::new(id)));
                }
            }
            "OBJECT_ARRAY" => {
                let values: Vec<Value> = serde_json::from_str(self.value.get())?;
                for (ordinal, value) in values.iter().enumerate() {
                    if let Some(id) = value.get("node_id").and_then(Value::as_str) {
                        links.push((ordinal, EquipmentNodeId::new(id)));
                    }
                }
            }
            _ => {}
        }
        Ok(links)
    }
}

impl Node {
    /// The node's native metadata as JSON, without its properties.
    pub fn metadata(&self) -> Value {
        serde_json::json!({"node_id":self.node_id,"view":self.view,"class_name":self.class_name,
            "instance_name":self.instance_name,"native_instance_id":self.native_instance_id,
            "instance_identity_kind":self.instance_identity_kind,"resource_name":self.resource_name,
            "source_addons":self.source_addons,"ancestor_id":self.ancestor_id,
            "ancestor_resource_name":self.ancestor_resource_name,"selection":self.selection,
            "children":self.children,"declared_properties":self.declared_properties})
    }
}

/// A property of a node that names another resource.
#[derive(Debug, Deserialize, Serialize)]
pub struct Reference {
    /// The node holding the property.
    pub node_id: EquipmentNodeId,
    /// The property that holds the reference.
    pub property: String,
    /// The resource named.
    pub resource_name: String,
    /// What kind of reference it is.
    pub kind: String,
    /// How the export resolved the reference.
    pub method: String,
}

/// The value an RFC 6901 JSON pointer names inside `raw`, keeping its original tokens; an empty
/// pointer names `raw` itself.
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
                .required("array entry not found")?
        } else {
            let mut values: BTreeMap<String, Box<RawValue>> = serde_json::from_str(current.get())?;
            values.remove(&key).required("property not found")?
        };
    }
    Ok(current)
}

/// `value` escaped as one JSON pointer segment (`~` as `~0`, `/` as `~1`).
pub fn pointer_segment(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}
