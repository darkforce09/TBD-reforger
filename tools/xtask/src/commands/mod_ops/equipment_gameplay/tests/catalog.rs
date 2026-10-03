use super::*;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    root: PathBuf,
    input: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tbd-gameplay-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let input = root.join("equipment_vehicle_exports/gameplay/generations/test");
        let source = root.join("source");
        fs::create_dir_all(source.join("records/test/00")).unwrap();
        fs::create_dir_all(source.join("sources/test/00")).unwrap();
        let resource = "{0123456789ABCDEF}Prefabs/Pack.et";
        let fact = json!({"status":"present","value":0,"native_type":"SCALAR","native_unit":"kg","unit_evidence":"native getter documentation","origin":"declared","source":{"resource_name":resource,"node_id":"root","property":"Weight","method":"BaseContainer.Get"},"reason":null,"enum_values":[]});
        let snapshot = json!({"schema_version":2,"resource_id":"guid:0123456789ABCDEF","nodes":[{"node_id":"root","class_name":"ItemPhysicalAttributes","view":"effective","instance_name":"","native_instance_id":null,"instance_identity_kind":"exporter_structural_path","resource_name":resource,"source_addons":[],"ancestor_id":null,"children":[],"declared_properties":["Weight"],"properties":{"Weight":fact}}]});
        let bytes = serde_json::to_string(&snapshot)
            .unwrap()
            .replace("\"value\":0", "\"value\":1.0000000000000000001");
        fs::write(source.join("sources/test/00/item.json"), bytes).unwrap();
        fs::write(source.join("records/test/00/item.json"),json!({"resource_id":"guid:0123456789ABCDEF","resource_name":resource,"resource_guid":"0123456789ABCDEF","identity_kind":"resource_guid","source_addons":[],"names":[]}).to_string()).unwrap();
        fs::write(source.join("generation.json"),json!({"schema_version":2,"status":"completed","scope":"complete","generation_id":"source","started_at":"time","finished_at":"time","environment":{},"errors":[],"reader_verification":{"status":"passed"},"type_hierarchy":{"types":{}},"equipment_ids":["guid:0123456789ABCDEF"],"vehicle_ids":[],"resources":[{"resource_id":"guid:0123456789ABCDEF","resource_name":resource,"record_file":"records/test/00/item.json","source_file":"sources/test/00/item.json","domains":["equipment"]}]}).to_string()).unwrap();
        projection::project(
            &repository_layout::find_repository_root().unwrap(),
            &source,
            &input,
        )
        .unwrap();
        Self { root, input }
    }
    fn resource(&self) -> PathBuf {
        self.input.join("resources/test/00/item.json")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn gameplay_preserves_numeric_tokens_and_rejects_broken_publication() {
    // Projection, validation and publication resolve the repository root from the cwd, which the
    // chdir-ing tests move into scratch trees they later delete. Holding the cwd lock for the
    // whole case keeps every one of those resolutions on the checkout.
    crate::commands::platform::wave_execution::testcwd::resolve_under_lock(
        numeric_tokens_survive_and_broken_publications_are_refused,
    );
}

fn numeric_tokens_survive_and_broken_publications_are_refused() {
    let fixture = Fixture::new();
    let bytes = fs::read_to_string(fixture.resource()).unwrap();
    assert!(bytes.contains("1.0000000000000000001"));
    let report = validate(&fixture.input).unwrap();
    assert!(report.valid, "{:?}", report.errors);
    super::super::equipment_vehicle_export::publish_command(&fixture.input).unwrap();
    let publication = fixture.input.parent().unwrap().parent().unwrap();
    let pointer = fs::read(publication.join("current.json")).unwrap();
    let archive = flate2::read::GzDecoder::new(
        fs::File::open(publication.join("archives/test.tar.gz")).unwrap(),
    );
    let mut archive = tar::Archive::new(archive);
    let names: Vec<_> = archive
        .entries()
        .unwrap()
        .map(|e| e.unwrap().path().unwrap().into_owned())
        .collect();
    assert!(names.iter().any(|p| p.to_str() == Some("manifest.json")));
    assert!(
        names
            .iter()
            .any(|p| p.to_str() == Some("publication_receipt.json"))
    );
    let published = publication.join("published/test");
    assert!(validate(&published).unwrap().valid);
    super::super::equipment_vehicle_export::publish_command(&fixture.input).unwrap();
    assert_eq!(pointer, fs::read(publication.join("current.json")).unwrap());
    let receipt_path = published.join("publication_receipt.json");
    let receipt_bytes = fs::read(&receipt_path).unwrap();
    let mut receipt: Value = serde_json::from_slice(&receipt_bytes).unwrap();
    receipt["catalog_bytes"] = json!(0);
    fs::write(&receipt_path, receipt.to_string()).unwrap();
    assert!(!validate(&published).unwrap().valid);
    assert!(super::super::equipment_vehicle_export::publish_command(&fixture.input).is_err());
    assert_eq!(pointer, fs::read(publication.join("current.json")).unwrap());
    fs::write(receipt_path, receipt_bytes).unwrap();
    let mut resource: Value = serde_json::from_str(&bytes).unwrap();
    resource["nodes"][0]["children"] = json!(["missing"]);
    fs::write(fixture.resource(), resource.to_string()).unwrap();
    assert!(!validate(&fixture.input).unwrap().valid);
    assert!(super::super::equipment_vehicle_export::publish_command(&fixture.input).is_err());
    assert_eq!(pointer, fs::read(publication.join("current.json")).unwrap());
}

#[test]
fn gameplay_policy_retains_gameplay_controls_and_excludes_presentation() {
    let policy = policy::Policy::load(&crate::core::repository_root::test_repo_root()).unwrap();
    assert_eq!(policy.fields.len(), 11314);
    for (class, property, native) in [
        (
            "SCR_MortarShellGadgetComponent",
            "m_aChargeRingConfig",
            "VECTOR3_ARRAY",
        ),
        ("SCR_TurretControllerComponent", "LimitsHoriz", "VECTOR2"),
        ("SCR_CarControllerComponent", "DownShiftRpm", "SCALAR"),
        ("WeaponAttachmentAttributes", "AttachmentType", "OBJECT"),
        ("MagazineComponent", "AmmoMapping", "INTEGER_ARRAY"),
    ] {
        assert_ne!(
            policy.rule(class, property, native).unwrap().disposition,
            "exclude",
            "{class}.{property}"
        );
    }
    assert!(
        policy
            .rule("MagazineComponent", "UnreviewedValue", "SCALAR")
            .is_err()
    );
    assert!(!policy.selected("ActionsManagerComponent").unwrap());
    assert_eq!(
        policy
            .rule("SCR_CarControllerComponent", "EngineRumbleEffect", "OBJECT")
            .unwrap()
            .disposition,
        "exclude"
    );
}
