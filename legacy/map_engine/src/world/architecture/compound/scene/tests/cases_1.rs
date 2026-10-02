//! Role: Domain regression cases.
//! Position: `world/architecture/compound/scene/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::architecture::compound::transform::Rigid;

use super::*;

#[test]
fn instances_json_round_trips_camel_case() {
    let door = InstanceRecord {
        id: "door_int_left_01/leaf".into(),
        kind: InstanceKind::DoorLeaf,
        prefab: "Prefabs/Doors/Leaf.et".into(),
        blas: "blas/Leaf.bvh".into(),
        xob: Some("Assets/Doors/Leaf.xob".into()),
        local: LocalTransform::from_rigid(&Rigid::from_enfusion(
            [1.0, 0.0, -2.0],
            [0.0, 90.0, 0.0],
            1.0,
        )),
        door: Some(DoorRecord {
            angle_range_deg: -120.0,
            closed_angle_deg: 0.0,
            initial_angle_deg: 0.0,
            angle_range_explicit: true,
            opened_distance: None,
        }),
        cover: CoverTier::Full,
        source: PlacementSource::XobSocket,
        parent: Some("door_int_left_01".into()),
    };
    let file = InstancesFile {
        schema_version: INSTANCES_SCHEMA_VERSION.into(),
        prefab_id: "House".into(),
        resource_name: "Prefabs/Houses/House.et".into(),
        shell_bvh: "House.bvh".into(),
        instances: vec![
            door.clone(),
            InstanceRecord {
                id: "furn/T1".into(),
                kind: InstanceKind::Furniture,
                prefab: "Prefabs/Props/Table.et".into(),
                blas: "blas/Table.bvh".into(),
                xob: None,
                local: LocalTransform::identity(),
                door: None,
                cover: CoverTier::Low,
                source: PlacementSource::PrefabCoords,
                parent: None,
            },
        ],
        notes: vec![],
    };
    let json = serde_json::to_string_pretty(&file).unwrap();
    assert!(json.contains("\"schemaVersion\""), "{json}");
    assert!(json.contains("\"kind\": \"doorLeaf\""));
    assert!(json.contains("\"cover\": \"full\""));
    assert!(json.contains("\"source\": \"xobSocket\""));
    assert!(json.contains("\"angleRangeDeg\": -120.0"));
    assert!(!json.contains("openedDistance"), "None is skipped");
    assert!(!json.contains("\"notes\""), "empty notes are skipped");
    let back: InstancesFile = serde_json::from_str(&json).unwrap();
    assert_eq!(back, file);
    assert_eq!(back.blas_paths(), ["blas/Leaf.bvh", "blas/Table.bvh"]);

    let r = back.instances[0].local.rigid();
    let d = r.dir([1.0, 0.0, 0.0]);
    assert!(d[0].abs() < 1e-9 && (d[2] + 1.0).abs() < 1e-9);

    let minimal: LocalTransform =
        serde_json::from_str(r#"{"pos":[0,0,0],"quat":[0,0,0,1]}"#).unwrap();
    assert_eq!(minimal.scale, 1.0);
}
