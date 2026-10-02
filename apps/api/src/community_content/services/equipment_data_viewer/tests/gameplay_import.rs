use super::*;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn publish(root: &Path, id: &str, unreviewed: bool) {
    let directory = root.join("published").join(id);
    fs::create_dir_all(directory.join("resources")).unwrap();
    let definition = json!({"class_name":"Storage","property":"Amount","field_name":"amount","section":"storage","native_type":"SCALAR","native_unit":null,"unit_evidence":null,"method":"BaseContainer.Get","disposition":"retain_value","follow_reference":false,"object_base_class":null,"enum_values":[]});
    let node = |id: &str, value: Value| json!({"node_id":id,"class_name":"Storage","instance_name":"","native_instance_id":null,"instance_identity_kind":"exporter_structural_path","resource_name":"{A}Pack.et","source_addons":[],"view":"effective","ancestor_id":null,"ancestor_resource_name":"{P}Parent.et","selection":"retained","children":[],"declared_properties":["Amount"],"properties":{"Amount":{"definition_id":"f","status":"present","value":value,"origin":"declared","reason":null}}});
    let mut first = node("opaque/one", json!(0));
    first["children"] = json!(["opaque/other"]);
    let resource = json!({"document_type":"gameplay_resource","schema_version":1,"resource_id":"A","resource_name":"{A}Pack.et","root_node":"opaque/one","nodes":[first,node("opaque/other",json!(false))],"capabilities":{"storage":["opaque/one","opaque/other"]},"source_addons":[],"names":[],"references":[]});
    let index = json!({"schema_version":1,"resources":[{"resource_id":"A","resource_name":"{A}Pack.et","resource_file":"resources/item.json","domains":["equipment"]}]});
    let generation = json!({"schema_version":1,"dataset_kind":"gameplay","generation_id":id,"scope":"complete","status":"completed","errors":[]});
    let mut files = serde_json::Map::new();
    for (name, value) in [
        ("resources/item.json", resource),
        ("generation.json", generation),
        ("resource_index.json", index),
        (
            "field_definitions.json",
            json!({"schema_version":1,"fields":{"f":definition}}),
        ),
        ("native_types.json", json!({"types":{}})),
        (
            "selection_report.json",
            json!({"policy_version":1,"unreviewed":if unreviewed{json!(["new field"])}else{json!([])},"resource_count":1,"node_count":2,"fact_count":2}),
        ),
    ] {
        let bytes = value
            .to_string()
            .replace("\"value\":0", "\"value\":1.0000000000000000001")
            .into_bytes();
        fs::write(directory.join(name), &bytes).unwrap();
        files.insert(
            name.into(),
            json!({"bytes":bytes.len(),"sha256":source::manifest::digest(&bytes)}),
        );
    }
    let manifest=json!({"schema_version":1,"dataset_kind":"gameplay","generation_id":id,"resource_count":1,"files":files}).to_string();
    fs::write(directory.join("manifest.json"), &manifest).unwrap();
    fs::write(root.join("current.json"),json!({"schema_version":1,"dataset_kind":"gameplay","generation_id":id,"directory":format!("published/{id}"),"manifest_sha256":source::manifest::digest(manifest.as_bytes())}).to_string()).unwrap();
}

#[tokio::test]
async fn gameplay_import_preserves_raw_values_instances_and_previous_generation() {
    let temp = Temp(std::env::temp_dir().join(format!("gameplay-import-{}", uuid::Uuid::new_v4())));
    let source = temp.0.join("source");
    fs::create_dir_all(&source).unwrap();
    publish(&source, "valid", false);
    let service = Arc::new(EquipmentDataService::new(
        temp.0.join("dataset"),
        Some(source.clone()),
    ));
    importing::generation_import::poll(service.clone())
        .await
        .unwrap();
    let dataset = service.dataset("latest").await.unwrap();
    assert_eq!(dataset.overview["facts"], 2);
    assert_eq!(dataset.overview["nodes"], 2);
    assert_eq!(
        fs::read(dataset.root.join("resources/item.json")).unwrap(),
        fs::read(source.join("published/valid/resources/item.json")).unwrap()
    );
    let query = queries::ViewerQuery {
        resource_id: Some("A".into()),
        node_id: Some("opaque/one".into()),
        ..Default::default()
    };
    let properties = queries::source_inspection::inspect(
        service.clone(),
        dataset.clone(),
        query.clone(),
        "properties",
    )
    .await
    .unwrap();
    assert_eq!(
        properties["items"][0]["value_json"],
        "1.0000000000000000001"
    );
    let metadata: Value =
        serde_json::from_str(properties["items"][0]["metadata_json"].as_str().unwrap()).unwrap();
    assert_eq!(metadata["source"]["node_id"], "opaque/one");
    assert!(metadata["native_unit"].is_null());
    let cards = queries::resource_cards::list(service.clone(), dataset.clone(), query)
        .await
        .unwrap();
    assert_eq!(cards["total"], 2);
    assert_eq!(cards["items"][1]["facts"][0]["value_json"], "false");
    publish(&source, "unreviewed", true);
    assert!(
        importing::generation_import::poll(service.clone())
            .await
            .is_err()
    );
    assert_eq!(service.current_id().as_deref(), Some("valid"));
    let catalogs = EquipmentDatasets::new(temp.0.join("catalogs"), Some(source));
    assert_ne!(
        catalogs.select(Some("diagnostic")).unwrap().data_dir,
        catalogs.select(None).unwrap().data_dir
    );
    assert!(catalogs.select(Some("unrecognized")).is_err());
}
