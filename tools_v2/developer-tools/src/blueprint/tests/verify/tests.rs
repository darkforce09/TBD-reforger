use super::*;
use website_map_engine::world::architecture::compound::assembly::CoverTier;
use website_map_engine::world::architecture::compound::instances::LocalTransform;

fn inst(
    id: &str,
    kind: InstanceKind,
    pos: [f64; 3],
    yaw: f64,
    parent: Option<&str>,
) -> InstanceRecord {
    InstanceRecord {
        id: id.into(),
        kind,
        prefab: format!("Prefabs/{id}.et"),
        blas: "blas/x.bvh".into(),
        xob: None,
        local: LocalTransform::from_rigid(&Rigid::from_enfusion(pos, [0.0, yaw, 0.0], 1.0)),
        door: None,
        cover: CoverTier::None,
        source: PlacementSource::XobSocket,
        parent: parent.map(str::to_string),
    }
}

fn child(depth: u32, class: &str, comps: &[&str], rel: [f64; 3], yaw: f64) -> ReconChild {
    ReconChild {
        depth,
        name: String::new(),
        class: class.into(),
        resource: String::new(),
        rel_pos: rel,
        yaw_deg: yaw,
        size: [1.0, 1.0, if class == "Building" { 0.5 } else { 0.0 }],
        components: comps.iter().map(|s| s.to_string()).collect(),
        pivot_id: String::new(),
        local_pos: None,
        angles_deg: None,
        world_pos: None,
        door: None,
    }
}

fn file(instances: Vec<InstanceRecord>) -> InstancesFile {
    InstancesFile {
        schema_version: "1.0.0".into(),
        prefab_id: "T".into(),
        resource_name: "Prefabs/T.et".into(),
        shell_bvh: "T.bvh".into(),
        instances,
        notes: vec![],
    }
}

#[test]
fn recon_groups_from_class_and_components() {
    assert_eq!(
        child(1, "Building", &["MeshObject"], [0.0; 3], 0.0).group(),
        Group::WindowFrame
    );
    assert_eq!(
        child(1, "GenericEntity", &["DoorSlotComponent"], [0.0; 3], 0.0).group(),
        Group::DoorFrame
    );
    assert_eq!(
        child(2, "GenericEntity", &["DoorComponent"], [0.0; 3], 0.0).group(),
        Group::DoorLeaf
    );
    assert_eq!(
        child(
            2,
            "GenericEntity",
            &["SCR_DestructionMultiPhaseComponent"],
            [0.0; 3],
            0.0
        )
        .group(),
        Group::Glass
    );
    assert_eq!(
        child(1, "StaticModelEntity", &[], [0.0; 3], 0.0).group(),
        Group::Prop
    );
    assert_eq!(wrap_deg(370.0), 10.0);
    assert_eq!(wrap_deg(-190.0), 170.0);
}

#[test]
fn matches_through_the_building_yaw_and_skips_furniture_descendants() {
    // Building yawed 90° in the world: a local child sits at world offset R_y(90)·local.
    let root_yaw = 90.0;
    let world = |p: [f64; 3]| Rigid::from_enfusion([0.0; 3], [0.0, root_yaw, 0.0], 1.0).point(p);
    let f = file(vec![
        inst(
            "win_1",
            InstanceKind::WindowFrame,
            [2.0, 0.0, -1.0],
            30.0,
            None,
        ),
        inst(
            "win_2",
            InstanceKind::WindowFrame,
            [-3.0, 1.0, 4.0],
            -90.0,
            None,
        ),
        inst(
            "door_1",
            InstanceKind::DoorFrame,
            [0.5, 0.0, 6.0],
            0.0,
            None,
        ),
        inst(
            "door_1/leaf",
            InstanceKind::DoorLeaf,
            [0.9, 0.0, 6.0],
            0.0,
            Some("door_1"),
        ),
        inst(
            "cupboard",
            InstanceKind::Furniture,
            [1.0, 0.0, 1.0],
            0.0,
            None,
        ),
        inst(
            "cupboard/pane",
            InstanceKind::Glass,
            [1.0, 1.0, 1.0],
            0.0,
            Some("cupboard"),
        ),
    ]);
    let recon = ReconFile {
        slug: "T".into(),
        root_angles: [0.0, root_yaw, 0.0],
        root_world_pos: None,
        children: vec![
            child(
                1,
                "Building",
                &["MeshObject"],
                world([2.0, 0.0, -1.0]),
                root_yaw + 30.0,
            ),
            child(
                1,
                "Building",
                &["MeshObject"],
                world([-3.0, 1.0, 4.0]),
                root_yaw - 90.0,
            ),
            child(
                1,
                "GenericEntity",
                &["DoorSlotComponent"],
                world([0.5, 0.0, 6.0]),
                root_yaw,
            ),
            child(
                2,
                "GenericEntity",
                &["DoorComponent"],
                world([0.9, 0.0, 6.0]),
                root_yaw,
            ),
        ],
    };
    let r = verify(&f, &recon);
    assert_eq!(r.yaw_sign, 1.0);
    assert_eq!(r.matches.len(), 4, "{r:?}");
    assert!(r.failures().is_empty(), "{:?}", r.matches);
    assert!(r.unmatched.is_empty());
    assert!(r.extra.is_empty());
    assert_eq!(r.skipped_furniture, 2, "cupboard + its pane");
    assert!(r.ok());
    // A displaced child is reported; a missing child leaves the instance unmatched.
    let mut moved = recon;
    moved.children[0].rel_pos[1] += 0.05;
    moved.children.pop();
    let r = verify(&f, &moved);
    assert_eq!(r.failures().len(), 1);
    assert_eq!(
        r.unmatched,
        vec!["door_1/leaf (DoorLeaf, Prefabs/door_1/leaf.et)"]
    );
    assert!(!r.ok());
}

/// The socket pin: the committed farmhouse instances against the committed
/// Workbench recon dump (88 architectural children, 2026-09-03).
#[test]
fn farmhouse_sockets_match_the_workbench_recon() {
    let root = crate::repository_paths::test_repo_root();
    let instances = crate::repository_layout::terrain_dir(&root, "everon")
        .join("prefabs/buildings/FarmHouse_E_1L01_Wood.instances.json");
    let recon = root.join(
        "tools_v2/developer-tools/test_fixtures/blueprint/FarmHouse_E_1L01_Wood_children.json",
    );
    let (file, dump) = load(&instances, &recon).unwrap();
    assert_eq!(dump.children.len(), 88);
    let r = verify(&file, &dump);
    assert_eq!(
        r.yaw_sign, 1.0,
        "handedness pin: Rigid::from_enfusion yaw sign"
    );
    assert_eq!(
        r.skipped_furniture, 2,
        "the two cupboard panes live under the furniture composition"
    );
    assert_eq!(r.matches.len(), 88, "unmatched: {:?}", r.unmatched);
    assert!(
        r.extra.is_empty(),
        "unclaimed recon children: {:?}",
        r.extra
    );
    assert!(
        r.failures().is_empty(),
        "worst pos {:.4} m · worst yaw {:.3}° · {:?}",
        r.worst_pos_m(),
        r.worst_yaw_deg(),
        r.failures()
    );
    // The enriched dump (Workbench restarted 2026-09-04): every leaf's hinge params, every
    // child's socket name and every nested child's parent-frame origin agree with the
    // prefab + XOB decode.
    assert_eq!(r.door_checks, 7);
    assert_eq!(r.pivot_checks, 88);
    assert!(r.local_checks >= 60, "{}", r.local_checks);
    assert!(r.door_mismatches.is_empty(), "{:?}", r.door_mismatches);
    assert!(r.pivot_mismatches.is_empty(), "{:?}", r.pivot_mismatches);
    assert!(r.local_mismatches.is_empty(), "{:?}", r.local_mismatches);
    assert!(r.ok());
}
