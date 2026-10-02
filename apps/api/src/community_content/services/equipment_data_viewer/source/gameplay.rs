use super::{
    document::Snapshot,
    manifest::{self, ResourceEntry},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

pub type FieldDefinitions = BTreeMap<String, Value>;

pub fn definitions(root: &Path, gameplay: bool) -> Result<FieldDefinitions> {
    if !gameplay {
        return Ok(BTreeMap::new());
    }
    let document: Value = serde_json::from_slice(&manifest::read(root, "field_definitions.json")?)?;
    ensure!(
        document["schema_version"] == 1,
        "unsupported gameplay definitions"
    );
    Ok(serde_json::from_value(document["fields"].clone())?)
}

pub fn entries(root: &Path, generation: &Value, gameplay: bool) -> Result<Vec<ResourceEntry>> {
    if !gameplay {
        return Ok(serde_json::from_value(generation["resources"].clone())?);
    }
    let document: Value = serde_json::from_slice(&manifest::read(root, "resource_index.json")?)?;
    document["resources"]
        .as_array()
        .context("gameplay resource index")?
        .iter()
        .map(|r| {
            let file = r["resource_file"]
                .as_str()
                .context("gameplay resource file")?
                .to_owned();
            Ok(ResourceEntry {
                resource_id: r["resource_id"]
                    .as_str()
                    .context("resource identity")?
                    .into(),
                resource_name: r["resource_name"].as_str().context("resource name")?.into(),
                record_file: file.clone(),
                source_file: file,
                domains: serde_json::from_value(r["domains"].clone())?,
            })
        })
        .collect()
}

pub fn snapshot(bytes: &[u8], definitions: &FieldDefinitions) -> Result<Snapshot> {
    let mut snapshot: Snapshot = serde_json::from_slice(bytes)?;
    if snapshot.schema_version == 2 {
        return Ok(snapshot);
    }
    ensure!(
        snapshot.schema_version == 1 && !definitions.is_empty(),
        "unsupported resource format"
    );
    let header: Value = serde_json::from_slice(bytes)?;
    ensure!(
        header["document_type"] == "gameplay_resource",
        "not a gameplay resource"
    );
    let resource_name = header["resource_name"].as_str().context("resource name")?;
    for node in &mut snapshot.nodes {
        for (property, fact) in &mut node.properties {
            let definition_id = fact.text("definition_id");
            let definition = definitions
                .get(definition_id)
                .context("missing shared field definition")?;
            ensure!(
                definition["class_name"] == node.class_name && definition["property"] == *property,
                "field definition ownership mismatch"
            );
            for (key, value) in definition.as_object().context("field metadata")? {
                fact.metadata.insert(key.clone(), value.clone());
            }
            fact.metadata.insert("source".into(),json!({"resource_name":resource_name,"node_id":node.node_id,"property":property,"method":definition["method"]}));
        }
    }
    Ok(snapshot)
}

pub fn record(bytes: &[u8], source: &Snapshot) -> Result<Value> {
    let mut record: Value = serde_json::from_slice(bytes)?;
    if source.schema_version == 2 {
        return Ok(record);
    }
    let sections = record["capabilities"]
        .as_object()
        .context("gameplay sections")?
        .clone();
    let nodes: BTreeMap<_, _> = source
        .nodes
        .iter()
        .map(|n| (n.node_id.as_str(), n))
        .collect();
    for (section, ids) in sections {
        let instances = ids
            .as_array()
            .context("section node identities")?
            .iter()
            .map(|id| {
                let node = nodes
                    .get(id.as_str().context("section node")?)
                    .context("missing section node")?;
                let mut facts = serde_json::Map::new();
                for (property, fact) in &node.properties {
                    if fact.text("section") == section {
                        facts.insert(
                            fact.text("field_name").into(),
                            json!({"source":{"property":property}}),
                        );
                    }
                }
                Ok(json!({"node_id":node.node_id,"facts":facts}))
            })
            .collect::<Result<Vec<_>>>()?;
        record["capabilities"][section] = json!(instances);
    }
    Ok(record)
}
