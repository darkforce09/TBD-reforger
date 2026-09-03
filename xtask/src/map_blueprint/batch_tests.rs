//! Tests for [`super`] (the BLAS / instance walker) — split out per the `#[path]`
//! precedent to stay under the SIZE gate.

use super::*;
use crate::map_blueprint::tests::fixture;

/// The real farmhouse closure out of the operator's paks — the T-090.11.2 acceptance pin
/// on the live install: 181 collision-bearing instances, 90 of them on decoded sockets, the
/// shell all-Opaque, every glass pane Glass. Needs `~/.cache/enfusion-mcp-root/addons`.
#[test]
#[ignore = "needs ~/.cache/enfusion-mcp-root/addons"]
fn real_farmhouse_closure_counts_are_pinned() {
    let Some(dir) = PakSet::default_dir().filter(|d| d.is_dir()) else {
        return;
    };
    let source = open_sources(Some(&dir), None).expect("pak set");
    let mut w = Walker::new(&source);
    let prefab = "Prefabs/Structures/Houses/Farm/FarmHouse_E_1L01/FarmHouse_E_1L01_Wood.et";
    let shell = w
        .walk(
            prefab,
            "FarmHouse_E_1L01_Wood",
            None,
            Rigid::identity(),
            PlacementSource::PrefabCoords,
            true,
            0,
        )
        .expect("walk")
        .expect("shell");
    assert_eq!(shell.kind_counts(), (4012, 0, 0));
    assert_eq!(w.instances.len(), 181, "instances");
    let by_kind = |k: InstanceKind| w.instances.iter().filter(|i| i.kind == k).count();
    assert_eq!(by_kind(InstanceKind::DoorFrame), 7);
    assert_eq!(by_kind(InstanceKind::DoorLeaf), 7);
    assert_eq!(by_kind(InstanceKind::WindowFrame), 16);
    assert_eq!(by_kind(InstanceKind::Glass), 58);
    assert_eq!(by_kind(InstanceKind::Furniture), 49);
    assert_eq!(by_kind(InstanceKind::Prop), 44);
    assert_eq!(
        w.instances
            .iter()
            .filter(|i| i.source == PlacementSource::XobSocket)
            .count(),
        90
    );
    for inst in w.instances.iter().filter(|i| i.kind == InstanceKind::Glass) {
        let a = w.assets.load(inst.xob.as_deref().unwrap()).unwrap();
        let (o, g, f) = a.kind_counts();
        assert!(o == 0 && f == 0 && g > 0, "{}: {:?}", inst.id, (o, g, f));
    }
    for inst in w
        .instances
        .iter()
        .filter(|i| i.kind == InstanceKind::DoorLeaf)
    {
        assert!(inst.door.is_some(), "{} has no door record", inst.id);
    }
}

#[test]
fn cover_and_kind_heuristics() {
    assert_eq!(
        cover_for_prefab("Prefabs/Props/Furniture/Cupboard_01/Cupboard_01_F.et"),
        (true, CoverTier::Full)
    );
    assert_eq!(
        cover_for_prefab("Prefabs/Props/Furniture/Table_01.et"),
        (true, CoverTier::Low)
    );
    assert_eq!(
        cover_for_prefab("Prefabs/Props/Furniture/Chair_02/Chair_02_green.et"),
        (true, CoverTier::None)
    );
    assert_eq!(
        cover_for_prefab("Prefabs/Props/Civilian/PaintCan_01.et"),
        (false, CoverTier::None)
    );
    assert_eq!(
        cover_for_prefab("Prefabs/Props/Crates/BoxWooden_01.et"),
        (true, CoverTier::Low)
    );
    assert_eq!(
        cover_for_prefab("Prefabs/Props/Furniture/LightWall_01.et"),
        (true, CoverTier::None)
    );
}

/// The synthetic prefab fixtures walked with a loose-directory source that also carries
/// tiny XOBs (built by the xob test helpers): sockets from the node table, coords from
/// the prefab text, kinds from the fake game materials.
#[test]
fn walker_places_door_set_window_and_furniture_from_fixtures() {
    use crate::map_blueprint::xob_nodes::XobNode;
    use crate::map_blueprint::xob_nodes::tests::{synth_head, wrap_xob};
    let dir = std::env::temp_dir().join(format!("tbd-batch-walk-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    // Copy the prefab fixtures, then add models.
    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for e in fs::read_dir(from).unwrap().flatten() {
            let p = e.path();
            let t = to.join(e.file_name());
            if p.is_dir() {
                copy_tree(&p, &t);
            } else {
                fs::copy(&p, &t).unwrap();
            }
        }
    }
    copy_tree(&fixture("prefab"), &dir);
    let s = 0.5f32.sqrt();
    let none = 0xFFFFu16;
    let rec = |name_idx: u32, pos: [f32; 3], quat: [f32; 4], next: u16, child: u16| XobNode {
        name_idx,
        pos,
        quat,
        next_sibling: next,
        first_child: child,
    };
    // House model: root + two sockets; one collider record whose material is wood, plus a
    // COLL chunk built by the xob test helper.
    let strings = [
        "Mat",
        "{A}Assets/a.emat",
        "Scene_Root",
        "socket_door_left_01",
        "socket_win_01",
        "socket_win_02",
        "Building",
        "UTM_BD_House",
        "{B}Common/Materials/Game/wood.gamemat",
        "{C}Common/Materials/Game/glass.gamemat",
    ];
    // Name space starts at "Mat": Scene_Root = 2, sockets 3..5, "Building" = 6, wood = 8.
    let house_nodes = [
        rec(2, [0.0; 3], [0.0, 0.0, 0.0, 1.0], none, 1),
        rec(3, [3.0, 0.0, -4.0], [0.0, s, 0.0, s], 2, none),
        rec(4, [-2.0, 1.0, 5.0], [0.0, 0.0, 0.0, 1.0], 3, none),
        rec(5, [2.0, 1.0, 5.0], [0.0, 0.0, 0.0, 1.0], none, none),
    ];
    let house = crate::map_blueprint::xob::tests::with_coll(
        wrap_xob(&synth_head(&strings, &house_nodes)),
        &crate::map_blueprint::xob::tests::coll_box_record_with_material(
            [0.0, 1.5, 0.0],
            [5.0, 1.5, 6.0],
            6,
            8,
        ),
    );
    // Frame model: one socket "socket_door_LEFT" offset 0.5 m along +x, glass material.
    let frame_strings = [
        "Scene_Root",
        "socket_door_LEFT",
        "Door",
        "{D}Common/Materials/Game/wood.gamemat",
    ];
    let frame_nodes = [
        rec(0, [0.0; 3], [0.0, 0.0, 0.0, 1.0], none, 1),
        rec(1, [0.5, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], none, none),
    ];
    let frame = crate::map_blueprint::xob::tests::with_coll(
        wrap_xob(&synth_head(&frame_strings, &frame_nodes)),
        &crate::map_blueprint::xob::tests::coll_box_record_with_material(
            [0.5, 1.0, 0.0],
            [0.6, 1.0, 0.1],
            2,
            3,
        ),
    );
    let leaf = crate::map_blueprint::xob::tests::with_coll(
        wrap_xob(&synth_head(
            &[
                "Scene_Root",
                "Door",
                "{E}Common/Materials/Game/wood.gamemat",
            ],
            &[rec(0, [0.0; 3], [0.0, 0.0, 0.0, 1.0], none, none)],
        )),
        &crate::map_blueprint::xob::tests::coll_box_record_with_material(
            [0.45, 1.0, 0.0],
            [0.45, 1.0, 0.03],
            1,
            2,
        ),
    );
    let win = crate::map_blueprint::xob::tests::with_coll(
        wrap_xob(&synth_head(
            &[
                "Scene_Root",
                "socket_glass_001",
                "socket_glass_002",
                "Building",
                "{F}Common/Materials/Game/wood.gamemat",
            ],
            &[
                rec(0, [0.0; 3], [0.0, 0.0, 0.0, 1.0], none, 1),
                rec(1, [-0.3, 0.7, 0.0], [0.0, 0.0, 0.0, 1.0], 2, none),
                rec(2, [0.3, 0.7, 0.0], [0.0, 0.0, 0.0, 1.0], none, none),
            ],
        )),
        &crate::map_blueprint::xob::tests::coll_box_record_with_material(
            [0.0, 0.7, 0.0],
            [0.6, 0.7, 0.05],
            3,
            4,
        ),
    );
    let glass = crate::map_blueprint::xob::tests::with_coll(
        wrap_xob(&synth_head(
            &[
                "Scene_Root",
                "Glass",
                "{G}Common/Materials/Game/glass.gamemat",
            ],
            &[rec(0, [0.0; 3], [0.0, 0.0, 0.0, 1.0], none, none)],
        )),
        &crate::map_blueprint::xob::tests::coll_box_record_with_material(
            [0.0, 0.0, 0.0],
            [0.25, 0.5, 0.01],
            1,
            2,
        ),
    );
    let table = crate::map_blueprint::xob::tests::with_coll(
        wrap_xob(&synth_head(
            &[
                "Scene_Root",
                "Prop",
                "{H}Common/Materials/Game/wood.gamemat",
            ],
            &[rec(0, [0.0; 3], [0.0, 0.0, 0.0, 1.0], none, none)],
        )),
        &crate::map_blueprint::xob::tests::coll_box_record_with_material(
            [0.0, 0.4, 0.0],
            [0.8, 0.4, 0.5],
            1,
            2,
        ),
    );
    for (rel, bytes) in [
        ("Assets/Houses/House.xob", &house),
        ("Assets/Doors/DoorFrame.xob", &frame),
        ("Assets/Doors/Door_Leaf.xob", &leaf),
        ("Assets/Windows/Win.xob", &win),
        ("Assets/Windows/Glass_01.xob", &glass),
        ("Assets/Props/Table.xob", &table),
        ("Assets/Props/Chair.xob", &table),
    ] {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, bytes).unwrap();
    }
    let src = DirSource { root: dir.clone() };
    let mut w = Walker::new(&src);
    let shell = w
        .walk(
            "Prefabs/Houses/House_Wood.et",
            "House_Wood",
            None,
            Rigid::identity(),
            PlacementSource::PrefabCoords,
            true,
            0,
        )
        .expect("walk")
        .expect("shell asset");
    assert!(shell.has_collision());
    assert_eq!(shell.kind_counts(), (12, 0, 0));
    let ids: Vec<(&str, InstanceKind, PlacementSource)> = w
        .instances
        .iter()
        .map(|i| (i.id.as_str(), i.kind, i.source))
        .collect();
    assert_eq!(
        ids,
        vec![
            (
                "socket_door_left_01",
                InstanceKind::DoorFrame,
                PlacementSource::XobSocket
            ),
            (
                "socket_door_left_01/socket_door_LEFT",
                InstanceKind::DoorLeaf,
                PlacementSource::XobSocket
            ),
            (
                "socket_win_01",
                InstanceKind::WindowFrame,
                PlacementSource::XobSocket
            ),
            (
                "socket_win_01/socket_glass_001",
                InstanceKind::Glass,
                PlacementSource::XobSocket
            ),
            (
                "socket_win_01/socket_glass_002",
                InstanceKind::Glass,
                PlacementSource::XobSocket
            ),
            (
                "socket_win_02",
                InstanceKind::WindowFrame,
                PlacementSource::XobSocket
            ),
            (
                "socket_win_02/socket_glass_001",
                InstanceKind::Glass,
                PlacementSource::XobSocket
            ),
            (
                "socket_win_02/socket_glass_002",
                InstanceKind::Glass,
                PlacementSource::XobSocket
            ),
            (
                "F1/T1",
                InstanceKind::Furniture,
                PlacementSource::PrefabCoords
            ),
            (
                "F1/C1",
                InstanceKind::Furniture,
                PlacementSource::PrefabCoords
            ),
            (
                "F1/C2",
                InstanceKind::Furniture,
                PlacementSource::PrefabCoords
            ),
        ]
    );
    // Door set sits on the house socket (yaw 90 at (3, 0, -4)) plus its own -0.1 z offset
    // (applied in the socket frame → world +x... the offset (0,0,-0.1) rotated by yaw 90
    // lands on -x).
    let set = &w.instances[0];
    let r = set.local.rigid();
    assert!(
        (r.t[0] - 2.9).abs() < 1e-6 && (r.t[2] + 4.0).abs() < 1e-6,
        "{:?}",
        r.t
    );
    assert!((r.yaw_deg() - 90.0).abs() < 1e-6);
    // The leaf hangs on the frame's socket 0.5 m along the frame's +x = world -z.
    let leaf = &w.instances[1];
    let rl = leaf.local.rigid();
    assert!(
        (rl.t[0] - 2.9).abs() < 1e-6 && (rl.t[2] + 4.5).abs() < 1e-6,
        "{:?}",
        rl.t
    );
    let d = leaf.door.as_ref().expect("door record");
    assert_eq!(d.angle_range_deg, -120.0);
    assert!(d.angle_range_explicit);
    assert_eq!(leaf.parent.as_deref(), Some("socket_door_left_01"));
    assert_eq!(leaf.blas, "blas/Door_Leaf.bvh");
    // Glass panes: glass material → Glass kind, all triangles glass.
    let pane = w.assets.load("Assets/Windows/Glass_01.xob").unwrap();
    assert_eq!(pane.kind_counts(), (0, 12, 0));
    assert_eq!(w.instances[3].cover, CoverTier::None);
    // Furniture: coords + yaw from the prefab; chair C2 keeps its scale.
    let t1 = &w.instances[8];
    assert!((t1.local.pos[0] - 1.035).abs() < 1e-9 && (t1.local.pos[2] + 7.666).abs() < 1e-9);
    assert!((t1.local.rigid().yaw_deg() - 91.667).abs() < 1e-6);
    assert_eq!(t1.cover, CoverTier::Low);
    assert_eq!(w.instances[9].cover, CoverTier::None);
    assert!((w.instances[10].local.scale - 1.152).abs() < 1e-12);
    // Two placements of one model share one BLAS; different models never do.
    assert_eq!(w.instances[9].blas, w.instances[10].blas);
    assert_eq!(w.instances[9].blas, "blas/Chair.bvh");
    assert_eq!(w.instances[8].blas, "blas/Table.bvh");
    // The probe child (no mesh) is a note, not an instance.
    assert!(w.notes.iter().any(|n| n.contains("Probe")), "{:?}", w.notes);
    let file = InstancesFile {
        schema_version: INSTANCES_SCHEMA_VERSION.into(),
        prefab_id: "House_Wood".into(),
        resource_name: "Prefabs/Houses/House_Wood.et".into(),
        shell_bvh: "House_Wood.bvh".into(),
        instances: w.instances.clone(),
        notes: w.notes.clone(),
    };
    let schema = crate::root::find_repo_root()
        .unwrap()
        .join("packages/tbd-schema/schema/building-instances.schema.json");
    validate_instances(&file, &schema).expect("schema-valid instances");
    fs::remove_dir_all(&dir).unwrap();
}
