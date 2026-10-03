use super::*;
use serde_json::{Value, json};
use source::manifest;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("equipment-viewer-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fact(value: Value, native: &str, node: &str, property: &str) -> Value {
    json!({"value":value,"status":"present","native_type":native,"origin":"declared","native_unit":null,"source":{"resource_name":"{A}Pack.et","node_id":node,"property":property,"method":"container.Get"}})
}
fn node(id: &str, view: &str, properties: Value) -> Value {
    json!({"node_id":id,"class_name":"Storage","view":view,"instance_name":"","native_instance_id":null,"instance_identity_kind":"structural_path","resource_name":"{A}Pack.et","source_addons":[],"ancestor_id":null,"children":[],"declared_properties":[],"properties":properties})
}

fn publish(root: &Path, id: &str, bad: bool) {
    let dir = root.join("published").join(id);
    fs::create_dir_all(&dir).unwrap();
    let values = json!({"Zero":fact(json!(0),"FLOAT","opaque/root","Zero"),"False":fact(json!(false),"BOOL","opaque/root","False"),"Empty":fact(json!([]),"INT_ARRAY","opaque/root","Empty"),"Objects":fact(json!([null,{"node_id":"same-class#2"},{"node_id":"same-class#2"}]),"OBJECT_ARRAY","opaque/root","Objects")});
    let mut first = node("opaque/root", "effective", values);
    first["ancestor_id"] = json!("parent:not/a/path");
    let source = json!({"schema_version":2,"resource_id":"A","root_node":"opaque/root","nodes":[node("same-class#2","effective",json!({"Zero":fact(json!(1),"FLOAT","same-class#2","Zero")})),first,node("parent:not/a/path","ancestor",json!({"Zero":fact(json!(12),"FLOAT","parent:not/a/path","Zero")}))]});
    let record = json!({"resource_id":"A","capabilities":{"storage":[{"node_id":"opaque/root","facts":{"zero":fact(json!(0),"FLOAT","opaque/root","Zero")}}]},"names":[],"references":[{"node_id":"opaque/root","property":"Objects","resource_name":"{A}Pack.et","kind":"gameplay","method":"container.Get"}],"source_addons":[]});
    let generation = json!({"generation_id":id,"schema_version":2,"scope":"complete","status":"completed","errors":[],"resources":[{"resource_id":"A","resource_name":"{A}Pack.et","record_file":"record.json","source_file":"source.json","domains":["equipment"]}]});
    let mut files = serde_json::Map::new();
    for (name, value) in [
        ("generation.json", generation),
        ("record.json", record),
        ("source.json", source),
    ] {
        let bytes = serde_json::to_vec(&value).unwrap();
        fs::write(dir.join(name), &bytes).unwrap();
        files.insert(
            name.into(),
            json!({"bytes":bytes.len(),"sha256":manifest::digest(&bytes)}),
        );
    }
    let bytes = serde_json::to_vec(
        &json!({"schema_version":2,"generation_id":id,"resource_count":1,"files":files}),
    )
    .unwrap();
    fs::write(dir.join("manifest.json"), &bytes).unwrap();
    fs::write(root.join("current.json"),serde_json::to_vec(&json!({"schema_version":2,"generation_id":id,"directory":format!("published/{id}"),"manifest_sha256":manifest::digest(&bytes)})).unwrap()).unwrap();
    if bad {
        fs::write(dir.join("source.json"), b"corrupted").unwrap();
    }
}

#[tokio::test]
async fn equipment_viewer_preserves_import_and_recovers_without_replacing_good_data() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    fs::create_dir(&source).unwrap();
    let destination = temp.0.join("dataset");
    publish(&source, "first", false);
    let service = Arc::new(EquipmentDataService::new(
        &destination,
        Some(source.clone()),
    ));
    importing::generation_import::poll(service.clone())
        .await
        .unwrap();
    let d = service.dataset(&"latest".into()).await.unwrap();
    assert_eq!(d.overview["facts"], 6);
    assert_eq!(d.overview["nodes"], 3);
    for file in [
        "record.json",
        "source.json",
        "generation.json",
        "manifest.json",
    ] {
        assert_eq!(
            fs::read(source.join("published/first").join(file)).unwrap(),
            fs::read(d.root.join(file)).unwrap()
        );
    }
    let p = queries::ViewerQuery {
        resource_id: Some("A".into()),
        ..Default::default()
    };
    let roots = queries::source_inspection::nodes(&d, &p).await.unwrap();
    assert_eq!(roots["items"][0]["node_id"], "opaque/root");
    let p = queries::ViewerQuery {
        resource_id: Some("A".into()),
        node_id: Some("opaque/root".into()),
        view: Some("all".into()),
        ..Default::default()
    };
    let edges = queries::source_inspection::nodes(&d, &p).await.unwrap();
    assert_eq!(edges["total"], 3);
    let properties =
        queries::source_inspection::inspect(service.clone(), d.clone(), p, "properties")
            .await
            .unwrap();
    assert!(
        properties["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["value_json"] == "false")
    );
    let mut organized_query = queries::ViewerQuery {
        resource_id: Some("A".into()),
        node_id: Some("opaque/root".into()),
        capability: Some("storage".into()),
        kind: Some("organized".into()),
        ..Default::default()
    };
    let organized = queries::source_inspection::inspect(
        service.clone(),
        d.clone(),
        organized_query.clone(),
        "properties",
    )
    .await
    .unwrap();
    assert_eq!(organized["total"], 1);
    assert_eq!(organized["items"][0]["key"], "Zero");
    assert_eq!(organized["items"][0]["value_json"], "0");
    assert_eq!(organized["items"][0]["label"], "Zero · zero");
    organized_query.node_id = Some("same-class#2".into());
    let other_instance = queries::source_inspection::inspect(
        service.clone(),
        d.clone(),
        organized_query,
        "properties",
    )
    .await
    .unwrap();
    assert_eq!(
        other_instance["total"], 0,
        "aliases belong to the exact source instance"
    );
    let fields = queries::fields::list(&d, &queries::ViewerQuery::default())
        .await
        .unwrap();
    assert_eq!(fields["total"], 4);
    let zero = fields["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["property"] == "Zero")
        .unwrap();
    assert_eq!(zero["effective_count"], 2);
    assert_eq!(zero["ancestor_count"], 1);
    assert_eq!(zero["resource_count"], 1);
    publish(&source, "bad", true);
    assert!(
        importing::generation_import::poll(service.clone())
            .await
            .is_err()
    );
    assert_eq!(
        service.current_id().as_ref().map(|id| id.as_str()),
        Some("first")
    );
    let restarted = EquipmentDataService::new(&destination, None);
    importing::generation_import::initialize(&restarted)
        .await
        .unwrap();
    assert_eq!(
        restarted.current_id().as_ref().map(|id| id.as_str()),
        Some("first")
    );
    publish(&source, "second", false);
    importing::generation_import::poll(service.clone())
        .await
        .unwrap();
    assert_eq!(
        service.current_id().as_ref().map(|id| id.as_str()),
        Some("second")
    );
    assert_eq!(
        service.dataset(&"first".into()).await.unwrap().overview["facts"],
        6
    );
}

#[test]
fn equipment_viewer_raw_values_and_unsafe_paths() {
    let raw = serde_json::value::RawValue::from_string(
        r#"{"native":[1.230000e-06,9007199254740993,0,false,[],null],"a/b~c":""}"#.into(),
    )
    .unwrap();
    assert_eq!(
        source::document::resolve_pointer(&raw, "/native/0")
            .unwrap()
            .get(),
        "1.230000e-06"
    );
    assert_eq!(
        source::document::resolve_pointer(&raw, "/native/1")
            .unwrap()
            .get(),
        "9007199254740993"
    );
    assert_eq!(
        source::document::resolve_pointer(&raw, "/a~1b~0c")
            .unwrap()
            .get(),
        "\"\""
    );
    let temp = Temp::new();
    assert!(manifest::safe_child(&temp.0, "../escape").is_err());
    assert!(manifest::safe_child(&temp.0, "/absolute").is_err());
    std::os::unix::fs::symlink("/tmp", temp.0.join("link")).unwrap();
    assert!(manifest::safe_child(&temp.0, "link/file").is_err());
}

#[tokio::test]
async fn equipment_viewer_interrupted_staging_locked_writer_and_activation_failure() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    fs::create_dir(&source).unwrap();
    let destination = temp.0.join("dataset");
    publish(&source, "first", false);
    let service = Arc::new(EquipmentDataService::new(
        &destination,
        Some(source.clone()),
    ));
    importing::generation_import::poll(service.clone())
        .await
        .unwrap();
    let current = fs::read(destination.join("current.json")).unwrap();
    publish(&source, "second", false);
    fs::create_dir_all(destination.join(".staging/index")).unwrap();
    fs::write(
        destination.join(".staging/index/catalog.sqlite"),
        b"interrupted index",
    )
    .unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(destination.join(".import.lock"))
        .unwrap();
    lock.lock().unwrap();
    assert!(
        importing::generation_import::poll(service.clone())
            .await
            .is_err()
    );
    drop(lock);
    fs::create_dir(destination.join(".current.json.tmp")).unwrap();
    assert!(
        importing::generation_import::poll(service.clone())
            .await
            .is_err()
    );
    assert_eq!(fs::read(destination.join("current.json")).unwrap(), current);
    assert_eq!(
        service.current_id().as_ref().map(|id| id.as_str()),
        Some("first")
    );
    fs::remove_dir(destination.join(".current.json.tmp")).unwrap();
    importing::generation_import::poll(service.clone())
        .await
        .unwrap();
    assert_eq!(
        service.current_id().as_ref().map(|id| id.as_str()),
        Some("second")
    );
    fs::remove_file(source.join("current.json")).unwrap();
    importing::generation_import::poll(service.clone())
        .await
        .unwrap();
    assert_eq!(
        service.current_id().as_ref().map(|id| id.as_str()),
        Some("second")
    );
}

#[test]
fn equipment_viewer_response_page_limits_preserve_continuations() {
    let items = (0..100)
        .map(|i| json!({"key":i,"metadata":"x".repeat(10_000)}))
        .collect();
    let page = queries::page("generation", 100, 0, items).unwrap();
    assert!(serde_json::to_vec(&page).unwrap().len() < PAGE_BYTES);
    let count = page["items"].as_array().unwrap().len();
    assert!(count > 0 && count < 100);
    assert_eq!(page["next_cursor"], count.to_string());
}

#[tokio::test]
async fn equipment_viewer_storage_failures_at_every_stage_keep_previous_generation() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    fs::create_dir(&source).unwrap();
    let destination = temp.0.join("dataset");
    publish(&source, "first", false);
    let service = Arc::new(EquipmentDataService::new(
        &destination,
        Some(source.clone()),
    ));
    importing::generation_import::poll(service.clone())
        .await
        .unwrap();
    let current = fs::read(destination.join("current.json")).unwrap();
    publish(&source, "second", false);
    for failing_stage in ["copying", "indexing", "activation"] {
        *service.test_hook.lock().unwrap() = Some(Box::new(move |stage| {
            if stage == failing_stage {
                return Err(std::io::Error::from(std::io::ErrorKind::StorageFull).into());
            }
            Ok(())
        }));
        let result = importing::generation_import::poll(service.clone()).await;
        assert!(matches!(
            result.unwrap_err(),
            error::Error::Filesystem(e) if e.kind() == std::io::ErrorKind::StorageFull
        ));
        assert_eq!(fs::read(destination.join("current.json")).unwrap(), current);
        assert_eq!(
            service
                .dataset(&"latest".into())
                .await
                .unwrap()
                .generation_id,
            "first"
        );
    }
    *service.test_hook.lock().unwrap() = None;
    importing::generation_import::poll(service.clone())
        .await
        .unwrap();
    assert_eq!(
        service.current_id().as_ref().map(|id| id.as_str()),
        Some("second")
    );
}

#[tokio::test]
async fn equipment_viewer_publication_during_import_is_picked_up_on_next_poll() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    fs::create_dir(&source).unwrap();
    publish(&source, "first", false);
    let service = Arc::new(EquipmentDataService::new(
        temp.0.join("dataset"),
        Some(source.clone()),
    ));
    let mut changed = false;
    *service.test_hook.lock().unwrap() = Some(Box::new(move |stage| {
        if stage == "copying" && !changed {
            publish(&source, "second", false);
            changed = true;
        }
        Ok(())
    }));
    importing::generation_import::poll(service.clone())
        .await
        .unwrap();
    assert_eq!(
        service.current_id().as_ref().map(|id| id.as_str()),
        Some("first")
    );
    *service.test_hook.lock().unwrap() = None;
    importing::generation_import::poll(service.clone())
        .await
        .unwrap();
    assert_eq!(
        service.current_id().as_ref().map(|id| id.as_str()),
        Some("second")
    );
}
