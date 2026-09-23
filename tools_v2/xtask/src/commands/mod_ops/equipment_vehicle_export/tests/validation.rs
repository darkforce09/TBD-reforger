use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

use super::{files, publication, validation};

const RESOURCE: &str = "{0123456789ABCDEF}Prefabs/Verification/Storage.et";
const ID: &str = "guid:0123456789ABCDEF";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    input: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tbd-source-export-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let input = root.join("equipment_vehicle_exports/generations/test_generation");
        fs::create_dir_all(input.join("records/test/01")).unwrap();
        fs::create_dir_all(input.join("sources/test/01")).unwrap();
        let fixture = Self { root, input };
        fixture.write("source", &snapshot());
        fixture.write("record", &record());
        fixture.write("generation", &generation());
        fixture
    }
    fn path(&self, kind: &str) -> PathBuf {
        self.input.join(match kind {
            "source" => "sources/test/01/asset.json",
            "record" => "records/test/01/asset.json",
            _ => "generation.json",
        })
    }
    fn write(&self, kind: &str, value: &Value) {
        fs::write(self.path(kind), serde_json::to_vec_pretty(value).unwrap()).unwrap();
    }
    fn assert_rejected(&self, needle: &str) {
        let report = validation::validate(&self.input).unwrap();
        assert!(!report.valid, "unexpected validation success");
        assert!(
            report.errors.iter().any(|e| e.contains(needle)),
            "{}",
            report.errors.join("\n")
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn fact() -> Value {
    json!({"status":"present","value":0,"native_type":"INTEGER","native_unit":null,"unit_evidence":null,"origin":"declared",
        "source":{"resource_name":RESOURCE,"node_id":"root","property":"MaxCumulativeVolume","method":"BaseContainer.Get"},"reason":null,"enum_values":[]})
}
fn snapshot() -> Value {
    json!({"document_type":"source_snapshot","schema_version":2,"resource_id":ID,"resource_name":RESOURCE,"root_node":"root",
        "nodes":[{"node_id":"root","view":"effective","class_name":"VerificationStorage","instance_name":"","native_instance_id":null,"instance_identity_kind":"exporter_structural_path","resource_name":RESOURCE,
            "source_addons":["Verification"],"ancestor_id":null,"children":[],"properties":{"MaxCumulativeVolume":fact()},"declared_properties":["MaxCumulativeVolume"]}]})
}
fn record() -> Value {
    json!({"document_type":"resource_record","schema_version":2,"resource_id":ID,"resource_name":RESOURCE,"resource_guid":"0123456789ABCDEF","identity_kind":"resource_guid",
        "source_addons":["Verification"],"capabilities":{"storage":[{"node_id":"root","facts":{"max_cumulative_volume":fact()}}]},"names":[],"references":[],"type_names":["VerificationStorage"]})
}
fn generation() -> Value {
    let metadata = json!({"status":"present","value":"verification-fixture","reason":null});
    json!({"document_type":"export_generation","schema_version":2,"generation_id":"test_generation","scope":"complete","status":"completed",
        "started_at":"2026-09-23T00:00:00Z","finished_at":"2026-09-23T00:00:01Z",
        "environment":{"game_build":metadata,"exporter_revision":metadata,
            "addon_load_order":[{"guid":"0123456789ABCDEF","addon_id":"Verification","title":"Verification fixture","version":metadata}],
            "settings":{"source_only":true,"locale":"en_us"}},
        "resources":[{"resource_id":ID,"resource_name":RESOURCE,"record_file":"records/test/01/asset.json","source_file":"sources/test/01/asset.json","domains":["equipment"]}],
        "equipment_ids":[ID],"vehicle_ids":[],"discovered_resources":[RESOURCE],"errors":[],
        "type_hierarchy":{"method":"TypeName.IsInherited","scope":"referenced_types_and_native_ancestors","types":{"VerificationStorage":{"status":"present","ancestor_types":[],"reason":null}}},"reader_verification":{"status":"passed","checks":["fixture"]}})
}

#[test]
fn explicit_zero_false_and_empty_values_are_preserved() {
    let f = Fixture::new();
    for (native, value) in [
        ("INTEGER", json!(0)),
        ("BOOLEAN", json!(false)),
        ("STRING", json!("")),
        ("STRING_ARRAY", json!([])),
    ] {
        let mut s = snapshot();
        let mut r = record();
        s["nodes"][0]["properties"]["MaxCumulativeVolume"]["native_type"] = json!(native);
        s["nodes"][0]["properties"]["MaxCumulativeVolume"]["value"] = value;
        r["capabilities"]["storage"][0]["facts"]["max_cumulative_volume"] =
            s["nodes"][0]["properties"]["MaxCumulativeVolume"].clone();
        f.write("source", &s);
        f.write("record", &r);
        let report = validation::validate(&f.input).unwrap();
        assert!(report.valid, "{:?}", report.errors);
    }
}

#[test]
fn replacing_child_zero_with_parent_default_fails() {
    let f = Fixture::new();
    let mut r = record();
    r["capabilities"]["storage"][0]["facts"]["max_cumulative_volume"]["value"] = json!(1100);
    f.write("record", &r);
    f.assert_rejected("differs from native source");
}

#[test]
fn same_class_distinct_instances_survive_but_duplicate_installations_fail() {
    let f = Fixture::new();
    let mut s = snapshot();
    let mut r = record();
    let mut child = s["nodes"][0].clone();
    child["node_id"] = json!("child");
    child["properties"] = json!({});
    child["declared_properties"] = json!([]);
    s["nodes"][0]["children"] = json!(["child"]);
    s["nodes"].as_array_mut().unwrap().push(child);
    r["capabilities"]["storage"]
        .as_array_mut()
        .unwrap()
        .push(json!({"node_id":"child","facts":{}}));
    f.write("source", &s);
    f.write("record", &r);
    assert!(validation::validate(&f.input).unwrap().valid);
    r["capabilities"]["storage"]
        .as_array_mut()
        .unwrap()
        .push(json!({"node_id":"child","facts":{}}));
    f.write("record", &r);
    f.assert_rejected("duplicate storage installation");
}

#[test]
fn ancestor_installations_cannot_leak_into_effective_capabilities() {
    let f = Fixture::new();
    let mut s = snapshot();
    s["nodes"][0]["view"] = json!("ancestor");
    f.write("source", &s);
    f.assert_rejected("outside the effective graph");
}

#[test]
fn missing_discovery_and_dependency_resources_fail() {
    let f = Fixture::new();
    let mut g = generation();
    g["discovered_resources"] = json!([RESOURCE, "missing"]);
    f.write("generation", &g);
    f.assert_rejected("discovered resource omitted");
    f.write("generation", &generation());
    let mut s = snapshot();
    let mut r = record();
    let mut reference = fact();
    reference["native_type"] = json!("RESOURCE_NAME");
    reference["value"] = json!("missing.conf");
    s["nodes"][0]["properties"]["MaxCumulativeVolume"] = reference.clone();
    r["capabilities"]["storage"][0]["facts"]["max_cumulative_volume"] = reference;
    r["references"] = json!([{"node_id":"root","property":"MaxCumulativeVolume","resource_name":"missing.conf","kind":"gameplay","method":"BaseContainer.Get"}]);
    f.write("source", &s);
    f.write("record", &r);
    f.assert_rejected("unresolved gameplay reference");
}

#[test]
fn cycles_and_unread_values_fail() {
    let f = Fixture::new();
    let mut s = snapshot();
    s["nodes"][0]["children"] = json!(["root"]);
    f.write("source", &s);
    f.assert_rejected("cyclic source graph");
    s = snapshot();
    s["nodes"][0]["properties"]["MaxCumulativeVolume"]["status"] = json!("error");
    s["nodes"][0]["properties"]["MaxCumulativeVolume"]["value"] = Value::Null;
    s["nodes"][0]["properties"]["MaxCumulativeVolume"]["reason"] = json!("read failed");
    f.write("source", &s);
    f.assert_rejected("read failed");
}

#[test]
fn duplicate_json_keys_and_unlisted_stale_files_fail() {
    let f = Fixture::new();
    fs::write(
        f.path("record"),
        r#"{"document_type":"resource_record","document_type":"resource_record"}"#,
    )
    .unwrap();
    f.assert_rejected("duplicate JSON key");
    f.write("record", &record());
    fs::write(f.input.join("old_optics.json"), "{}").unwrap();
    f.assert_rejected("unlisted generation file");
}

#[test]
fn path_traversal_and_symlinks_fail() {
    let f = Fixture::new();
    let mut g = generation();
    g["resources"][0]["source_file"] = json!("../outside.json");
    f.write("generation", &g);
    f.assert_rejected("unsafe export path");
    f.write("generation", &generation());
    fs::remove_file(f.path("source")).unwrap();
    std::os::unix::fs::symlink(f.path("record"), f.path("source")).unwrap();
    f.assert_rejected("symlink");
}

#[test]
fn partial_and_unverified_exports_cannot_publish() {
    let f = Fixture::new();
    let mut g = generation();
    g["scope"] = json!("diagnostic");
    f.write("generation", &g);
    assert!(publication::publish(&f.input).is_err());
    g["scope"] = json!("complete");
    g["reader_verification"]["status"] = json!("not_run");
    f.write("generation", &g);
    assert!(publication::publish(&f.input).is_err());
    assert!(
        !f.root
            .join("equipment_vehicle_exports/current.json")
            .exists()
    );
}

#[test]
fn publication_is_sealed_retryable_and_preserves_old_data() {
    let f = Fixture::new();
    fs::create_dir(f.root.join("equipment")).unwrap();
    fs::write(f.root.join("equipment/legacy.json"), "legacy").unwrap();
    publication::publish(&f.input).unwrap();
    let root = f.root.join("equipment_vehicle_exports");
    let before = fs::read(root.join("current.json")).unwrap();
    let sealed = root.join("published/test_generation");
    let (manifest, _) = files::read(&sealed, "manifest.json").unwrap();
    assert_eq!(
        manifest["files"]["generation.json"]["sha256"],
        files::digest(&fs::read(sealed.join("generation.json")).unwrap()).sha256
    );
    assert_eq!(
        fs::read_to_string(root.join("legacy/test_generation/equipment/legacy.json")).unwrap(),
        "legacy"
    );
    publication::publish(&f.input).unwrap();
    assert_eq!(fs::read(root.join("current.json")).unwrap(), before);
    let sealed_record = sealed.join("records/test/01/asset.json");
    let original_bytes = fs::read(&sealed_record).unwrap();
    fs::write(&sealed_record, b"tampered after publication").unwrap();
    assert!(publication::publish(&f.input).is_err());
    assert_eq!(fs::read(root.join("current.json")).unwrap(), before);
    fs::write(&sealed_record, original_bytes).unwrap();
    let mut r = record();
    r["capabilities"]["storage"][0]["facts"]["max_cumulative_volume"]["value"] = json!(5);
    f.write("record", &r);
    assert!(publication::publish(&f.input).is_err());
    assert_eq!(fs::read(root.join("current.json")).unwrap(), before);
    assert_eq!(
        files::read(&sealed, "records/test/01/asset.json")
            .unwrap()
            .0,
        record()
    );
}

#[test]
fn native_reference_cannot_be_silently_omitted() {
    let f = Fixture::new();
    let mut s = snapshot();
    let mut r = record();
    let native = &mut s["nodes"][0]["properties"]["MaxCumulativeVolume"];
    native["native_type"] = json!("RESOURCE_NAME");
    native["value"] = json!("{FEDCBA9876543210}Prefabs/Verification/Dependency.et");
    r["capabilities"]["storage"][0]["facts"]["max_cumulative_volume"] = native.clone();
    f.write("source", &s);
    f.write("record", &r);
    f.assert_rejected("relationships are omitted");
}

#[test]
fn a_readable_property_cannot_be_dismissed_as_unavailable() {
    let f = Fixture::new();
    let mut r = record();
    let field = &mut r["capabilities"]["storage"][0]["facts"]["max_cumulative_volume"];
    field["status"] = json!("unavailable");
    field["value"] = Value::Null;
    field["reason"] = json!("Pretend this property cannot be read");
    f.write("record", &r);
    f.assert_rejected("existing property marked absent");
}

#[test]
fn type_inventory_and_hierarchy_must_close() {
    let f = Fixture::new();
    let mut r = record();
    r["type_names"] = json!([]);
    f.write("record", &r);
    f.assert_rejected("type inventory is incomplete");
    f.write("record", &record());
    let mut g = generation();
    g["type_hierarchy"]["types"] = json!({});
    f.write("generation", &g);
    f.assert_rejected("native type is missing");
}

#[test]
fn finalized_hashes_detect_consistent_but_tampered_facts() {
    let f = Fixture::new();
    publication::publish(&f.input).unwrap();
    let sealed = f
        .root
        .join("equipment_vehicle_exports/published/test_generation");
    let mut s = snapshot();
    let mut r = record();
    s["nodes"][0]["properties"]["MaxCumulativeVolume"]["value"] = json!(123);
    r["capabilities"]["storage"][0]["facts"]["max_cumulative_volume"] =
        s["nodes"][0]["properties"]["MaxCumulativeVolume"].clone();
    fs::write(
        sealed.join("sources/test/01/asset.json"),
        serde_json::to_vec(&s).unwrap(),
    )
    .unwrap();
    fs::write(
        sealed.join("records/test/01/asset.json"),
        serde_json::to_vec(&r).unwrap(),
    )
    .unwrap();
    let report = validation::validate(&sealed).unwrap();
    assert!(!report.valid);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("manifest counts or file hashes"))
    );
}

#[test]
fn archive_failure_restores_every_legacy_directory() {
    let f = Fixture::new();
    fs::create_dir(f.root.join("equipment")).unwrap();
    fs::write(f.root.join("equipment/old.json"), b"old equipment").unwrap();
    fs::write(f.root.join("vehicles"), b"not a directory").unwrap();
    assert!(publication::publish(&f.input).is_err());
    assert_eq!(
        fs::read(f.root.join("equipment/old.json")).unwrap(),
        b"old equipment"
    );
    assert!(
        !f.root
            .join("equipment_vehicle_exports/current.json")
            .exists()
    );
}

#[test]
fn failed_validation_keeps_the_current_generation_byte_for_byte() {
    let f = Fixture::new();
    publication::publish(&f.input).unwrap();
    let current = f.root.join("equipment_vehicle_exports/current.json");
    let before = fs::read(&current).unwrap();
    fs::write(f.path("source"), b"{incomplete write").unwrap();
    assert!(publication::publish(&f.input).is_err());
    assert_eq!(fs::read(current).unwrap(), before);
}

#[test]
fn interrupted_archival_is_recovered_before_validation() {
    let f = Fixture::new();
    fs::create_dir(f.root.join("vehicles")).unwrap();
    fs::write(f.root.join("vehicles/old.json"), b"keep unmoved vehicles").unwrap();
    let root = f.root.join("equipment_vehicle_exports");
    let archived = root.join("legacy/interrupted/equipment");
    fs::create_dir_all(&archived).unwrap();
    fs::write(archived.join("old.json"), b"keep old data").unwrap();
    let journal = json!({"generation_id":"interrupted","entries":[
        {"source":"equipment","destination":"legacy/interrupted/equipment"},
        {"source":"vehicles","destination":"legacy/interrupted/vehicles"}
    ]});
    fs::write(
        root.join(".legacy-archive.json"),
        serde_json::to_vec(&journal).unwrap(),
    )
    .unwrap();
    fs::write(f.path("source"), b"incomplete").unwrap();
    assert!(publication::publish(&f.input).is_err());
    assert_eq!(
        fs::read(f.root.join("equipment/old.json")).unwrap(),
        b"keep old data"
    );
    assert!(!root.join(".legacy-archive.json").exists());
    assert!(!root.join("current.json").exists());
    assert_eq!(
        fs::read(f.root.join("vehicles/old.json")).unwrap(),
        b"keep unmoved vehicles"
    );
}

#[test]
fn native_enums_vectors_units_and_precision_survive_without_conversion() {
    let f = Fixture::new();
    let mut s = snapshot();
    let mut r = record();
    let native = &mut s["nodes"][0]["properties"]["MaxCumulativeVolume"];
    native["native_type"] = json!("VECTOR3");
    native["value"] = json!([0.125, 200.0, 94.5999984741211]);
    native["native_unit"] = json!("native_fixture_unit");
    native["unit_evidence"] = json!("test fixture specification");
    native["enum_values"] = json!([{"name":"EngineAuthoredName", "value":7}]);
    r["capabilities"]["storage"][0]["facts"]["max_cumulative_volume"] = native.clone();
    f.write("source", &s);
    f.write("record", &r);
    assert!(validation::validate(&f.input).unwrap().valid);
    r["capabilities"]["storage"][0]["facts"]["max_cumulative_volume"]["value"] =
        json!([0.125, 200.0, 94.6]);
    f.write("record", &r);
    f.assert_rejected("differs from native source");
}

#[test]
fn shared_ancestor_objects_cannot_replace_effective_objects() {
    let f = Fixture::new();
    let mut s = snapshot();
    let mut ancestor = s["nodes"][0].clone();
    ancestor["node_id"] = json!("ancestor");
    ancestor["view"] = json!("ancestor");
    ancestor["properties"] = json!({});
    ancestor["declared_properties"] = json!([]);
    s["nodes"][0]["children"] = json!(["ancestor"]);
    s["nodes"].as_array_mut().unwrap().push(ancestor);
    f.write("source", &s);
    f.assert_rejected("crosses effective/ancestor views");
    s["nodes"][0]["children"] = json!([]);
    s["nodes"][0]["ancestor_id"] = json!("ancestor");
    f.write("source", &s);
    assert!(validation::validate(&f.input).unwrap().valid);
}

#[test]
fn native_instance_identifiers_require_source_evidence() {
    let f = Fixture::new();
    let mut s = snapshot();
    s["nodes"][0]["native_instance_id"] = json!("{ABCDEF0123456789}");
    s["nodes"][0]["instance_identity_kind"] = json!("native_container_id");
    f.write("source", &s);
    f.assert_rejected("invalid native instance identity");
    s["nodes"][0]["resource_name"] = json!("{ABCDEF0123456789}");
    f.write("source", &s);
    assert!(validation::validate(&f.input).unwrap().valid);
}

#[test]
fn organized_capability_cannot_silently_omit_properties() {
    let f = Fixture::new();
    let mut r = record();
    r["capabilities"]["storage"][0]["facts"] = json!({});
    f.write("record", &r);
    f.assert_rejected("omits or duplicates native properties");
}
