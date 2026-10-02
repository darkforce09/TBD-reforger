//! Live API goldens claim every viewer wire field.
use super::equipment_data_viewer::*;
use super::r_api::assert_golden;

#[test]
fn equipment_viewer_dataset_parity() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../contracts/fixtures/equipment-data-viewer");
    let text = std::fs::read_to_string(root.join("positive/dataset.json")).unwrap();
    assert_golden::<EquipmentDatasetStatus>(&text, &[]);
    assert!(serde_json::from_str::<EquipmentDatasetStatus>(
        &std::fs::read_to_string(root.join("negative/dataset.json")).unwrap()
    )
    .is_err());
}
#[test]
fn equipment_viewer_resources_parity() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../contracts/fixtures/equipment-data-viewer");
    let text = std::fs::read_to_string(root.join("positive/resources.json")).unwrap();
    assert_golden::<EquipmentResourcePage>(&text, &[]);
    assert!(serde_json::from_str::<EquipmentResourcePage>(
        &std::fs::read_to_string(root.join("negative/resources.json")).unwrap()
    )
    .is_err());
}
#[test]
fn equipment_viewer_source_inspection_parity() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../contracts/fixtures/equipment-data-viewer");
    let text = std::fs::read_to_string(root.join("positive/source-inspection.json")).unwrap();
    assert_golden::<EquipmentSourcePage>(&text, &[]);
    assert!(serde_json::from_str::<EquipmentSourcePage>(
        &std::fs::read_to_string(root.join("negative/source-inspection.json")).unwrap()
    )
    .is_err());
}
#[test]
fn equipment_viewer_relationships_parity() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../contracts/fixtures/equipment-data-viewer");
    let text = std::fs::read_to_string(root.join("positive/relationships.json")).unwrap();
    assert_golden::<EquipmentRelationshipPage>(&text, &[]);
    assert!(serde_json::from_str::<EquipmentRelationshipPage>(
        &std::fs::read_to_string(root.join("negative/relationships.json")).unwrap()
    )
    .is_err());
}
#[test]
fn equipment_viewer_field_inventory_parity() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../contracts/fixtures/equipment-data-viewer");
    let text = std::fs::read_to_string(root.join("positive/field-inventory.json")).unwrap();
    assert_golden::<EquipmentFieldPage>(&text, &[]);
    assert!(serde_json::from_str::<EquipmentFieldPage>(
        &std::fs::read_to_string(root.join("negative/field-inventory.json")).unwrap()
    )
    .is_err());
}

#[test]
fn equipment_viewer_resource_cards_parity() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../contracts/fixtures/equipment-data-viewer");
    assert_golden::<EquipmentResourceCardPage>(
        &std::fs::read_to_string(root.join("positive/resource-cards.json")).unwrap(),
        &[],
    );
    assert!(serde_json::from_str::<EquipmentResourceCardPage>(
        &std::fs::read_to_string(root.join("negative/resource-cards.json")).unwrap()
    )
    .is_err());
}
