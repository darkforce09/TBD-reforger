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
    fs::write(f.root.join("equipment/unversioned.json"), "unversioned").unwrap();
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
        fs::read_to_string(
            root.join("unversioned_exports/test_generation/equipment/unversioned.json")
        )
        .unwrap(),
        "unversioned"
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
fn archive_failure_restores_every_unversioned_export_folder() {
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
