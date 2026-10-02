//! Resource titles use owning-item names; action and compartment names stay local.
use super::document::Snapshot;
use serde_json::Value;

pub fn resource_label(record: &Value, snapshot: &Snapshot, vehicle: bool) -> String {
    let root = snapshot
        .nodes
        .iter()
        .find(|n| n.node_id == snapshot.root_node);
    let mut eligible = Vec::new();
    if let Some(root) = root {
        let components = root
            .properties
            .get("components")
            .map(|f| {
                f.links()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(_, id)| id)
                    .collect()
            })
            .unwrap_or_else(|| root.children.clone());
        for id in components {
            if let Some(component) = snapshot.nodes.iter().find(|n| n.node_id == id) {
                let fields: &[&str] = if vehicle {
                    if component.class_name == "SCR_EditableVehicleComponent" {
                        &["UIInfo", "m_UIInfo"]
                    } else {
                        &[]
                    }
                } else {
                    &["Attributes"]
                };
                for field in fields {
                    if let Some(fact) = component.properties.get(*field) {
                        for (_, child) in fact.links().unwrap_or_default() {
                            if vehicle {
                                eligible.push(child);
                            } else if let Some(attributes) =
                                snapshot.nodes.iter().find(|n| n.node_id == child)
                                && let Some(display) = attributes.properties.get("ItemDisplayName")
                            {
                                eligible.extend(
                                    display
                                        .links()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(|(_, id)| id),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    let mut names = Vec::new();
    if let Some(entries) = record["names"].as_array() {
        for entry in entries {
            if eligible
                .iter()
                .any(|id| entry["node_id"].as_str() == Some(id))
                && entry["display_name_en"]["status"] == "present"
                && let Some(name) = entry["display_name_en"]["value"]
                    .as_str()
                    .filter(|s| !s.is_empty())
            {
                names.push(name.to_owned());
            }
        }
    }
    names.sort();
    names.dedup();
    if names.len() == 1 {
        return names.remove(0);
    }
    record["resource_name"]
        .as_str()
        .unwrap_or("Unnamed resource")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_owned()
}
