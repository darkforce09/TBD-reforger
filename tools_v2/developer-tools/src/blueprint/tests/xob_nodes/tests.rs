use super::*;

/// A HEAD payload: 48 header bytes (with a stray printable byte, like the shipped
/// files), a string table, filler, node records, one LZO4 tag.
pub(crate) fn synth_head(strings: &[&str], records: &[XobNode]) -> Vec<u8> {
    let mut head = vec![0u8; 48];
    head[40] = b'k';
    for s in strings {
        head.extend_from_slice(s.as_bytes());
        head.push(0);
    }
    head.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]); // binary filler the scan must stop at
    for r in records {
        head.extend_from_slice(&r.name_idx.to_le_bytes());
        for c in r.pos {
            head.extend_from_slice(&c.to_le_bytes());
        }
        for c in r.quat {
            head.extend_from_slice(&c.to_le_bytes());
        }
        head.extend_from_slice(&r.next_sibling.to_le_bytes());
        head.extend_from_slice(&r.first_child.to_le_bytes());
    }
    head.extend_from_slice(b"LZO4");
    head.extend_from_slice(&[0u8; 112]);
    head
}

pub(crate) fn wrap_xob(head: &[u8]) -> Vec<u8> {
    let mut file = Vec::new();
    file.extend_from_slice(b"FORM");
    file.extend_from_slice(&0u32.to_be_bytes());
    file.extend_from_slice(b"XOB9HEAD");
    file.extend_from_slice(&(head.len() as u32).to_be_bytes());
    file.extend_from_slice(head);
    let total = (file.len() - 8) as u32;
    file[4..8].copy_from_slice(&total.to_be_bytes());
    file
}

fn rec(name_idx: u32, pos: [f32; 3], quat: [f32; 4], next: u16, child: u16) -> XobNode {
    XobNode {
        name_idx,
        pos,
        quat,
        next_sibling: next,
        first_child: child,
    }
}

#[test]
fn node_table_decodes_sockets_and_the_name_space_starts_at_the_first_material() {
    // Two leading material strings + one empty entry precede Scene_Root (index 3).
    let strings = [
        "Mat_A",
        "{AAAA}Assets/a.emat",
        "",
        "Scene_Root",
        "socket_door_01",
        "Socket_Win_01",
        "Mesh_Group",
        "{BBBB}Common/Materials/Game/wood.gamemat",
    ];
    let s = (0.5f32).sqrt();
    let records = [
        rec(3, [0.0; 3], [0.0, 0.0, 0.0, 1.0], NONE, 1),
        rec(4, [2.0, 0.0, -1.0], [0.0, s, 0.0, s], 2, NONE), // yaw +90° about Y
        rec(5, [1.0, 1.5, 3.0], [0.0, 0.0, 0.0, 1.0], NONE, 3), // child below
        rec(6, [0.0, 0.5, 0.0], [0.0, 0.0, 0.0, 1.0], NONE, NONE), // Mesh_Group under the window
    ];
    let file = wrap_xob(&synth_head(&strings, &records));
    let nodes = parse_head_nodes(&file).expect("node table");
    assert_eq!(nodes.name_base, 48, "table starts after the header words");
    assert!(nodes.has_hierarchy());
    assert_eq!(nodes.nodes.len(), 4);
    assert_eq!(nodes.node_name(0), Some("Scene_Root"));
    assert_eq!(nodes.name(0), Some("Mat_A"));
    assert_eq!(
        nodes.name(7),
        Some("{BBBB}Common/Materials/Game/wood.gamemat")
    );
    assert_eq!(nodes.parent, vec![None, Some(0), Some(0), Some(2)]);
    let sockets = nodes.sockets();
    assert_eq!(sockets.len(), 2);
    assert_eq!(sockets[0].name, "socket_door_01");
    let door = &sockets[0].local;
    assert!((door.t[0] - 2.0).abs() < 1e-6 && (door.t[2] + 1.0).abs() < 1e-6);
    // Yaw +90° about Y maps local +x onto -z (Enfusion is left-handed: standard R_y).
    let px = door.dir([1.0, 0.0, 0.0]);
    assert!((px[0]).abs() < 1e-6 && (px[2] + 1.0).abs() < 1e-6, "{px:?}");
    // Case-insensitive lookup; the nested mesh group composes through its parent.
    assert_eq!(nodes.socket("SOCKET_WIN_01").unwrap().node, 2);
    let grp = nodes.world_of(3);
    assert!((grp.t[1] - 2.0).abs() < 1e-6 && (grp.t[2] - 3.0).abs() < 1e-6);
}

/// The acceptance on the real farmhouse: 26 records (root + 25 sockets), COLL
/// record 0's nine subranges resolve to the nine game materials in file order and cover
/// all 1129 triangles, record 1 (`FireView`) resolves too. Needs the operator's extract.
#[test]
#[ignore = "needs ~/ReforgerExtract/unpacked/…/FarmHouse_E_1L01.xob"]
fn real_farmhouse_nodes_sockets_and_materials() {
    let home = std::env::var("HOME").unwrap();
    let path = std::path::PathBuf::from(home).join(
        "ReforgerExtract/unpacked/Assets/Structures/Houses/Farm/FarmHouse_E_1L01/FarmHouse_E_1L01.xob",
    );
    let Ok(data) = std::fs::read(&path) else {
        return;
    };
    let nodes = parse_head_nodes(&data).expect("farmhouse node table");
    assert_eq!(nodes.nodes.len(), 26, "root + 25 sockets");
    assert_eq!(nodes.node_name(0), Some("Scene_Root"));
    let sockets = nodes.sockets();
    assert_eq!(sockets.len(), 25);
    let names: Vec<&str> = sockets.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"socket_door_ext_left_01"));
    assert!(names.contains(&"Socket_Win_50x75_01"));
    assert!(
        sockets
            .iter()
            .all(|s| s.local.t.iter().all(|c| c.abs() < 20.0))
    );
    let mesh = super::super::xob::parse_coll(&data).expect("coll");
    assert_eq!(mesh.records.len(), 2);
    assert_eq!(
        nodes.name(u32::from(mesh.records[0].layer_idx)),
        Some("Building")
    );
    assert_eq!(
        nodes.name(u32::from(mesh.records[1].layer_idx)),
        Some("FireView")
    );
    assert_eq!(
        nodes.name(u32::from(mesh.records[0].mesh_idx)),
        Some("UTM_BD_FarmHouse_E_1L01")
    );
    let r0 = &mesh.records[0];
    assert_eq!(r0.tri_count, 1129);
    let mut order: Vec<String> = Vec::new();
    for t in r0.tri_start..r0.tri_start + r0.tri_count {
        let m = mesh.tri_material[t];
        assert_ne!(m, u32::MAX, "triangle {t} has no material");
        let name = nodes.name(m).expect("material name").to_string();
        assert!(name.ends_with(".gamemat"), "{name}");
        if order.last() != Some(&name) {
            order.push(name);
        }
    }
    let stems: Vec<String> = order
        .iter()
        .map(|p| super::super::surface_kind::gamemat_stem(p))
        .collect();
    assert_eq!(
        stems,
        [
            "tiles_ceramic",
            "wood",
            "concrete",
            "carpet",
            "tiles",
            "metal",
            "stone",
            "wood_floor",
            "brick"
        ]
    );
    let r1 = &mesh.records[1];
    assert_eq!(r1.tri_count, 2883);
    assert!((r1.tri_start..r1.tri_start + r1.tri_count).all(|t| {
        nodes
            .name(mesh.tri_material[t])
            .is_some_and(|n| n.ends_with(".gamemat"))
    }));
}

/// A model without a hierarchy (door leaf, pane, tree) still yields its name space.
#[test]
fn hierarchy_less_models_keep_their_string_table() {
    let file = wrap_xob(&synth_head(
        &[
            "Doors_Village_E",
            "{X}a.emat",
            "Leaf",
            "DoorFireView",
            "UBX_Leaf",
            "{Y}wood.gamemat",
        ],
        &[],
    ));
    let nodes = parse_head_nodes(&file).expect("strings only");
    assert!(!nodes.has_hierarchy());
    assert!(nodes.sockets().is_empty());
    assert_eq!(nodes.name(3), Some("DoorFireView"));
    assert_eq!(nodes.name(5), Some("{Y}wood.gamemat"));
    // Records that do not start at Scene_Root are ignored, not trusted.
    let file = wrap_xob(&synth_head(
        &["Mat", "Other"],
        &[XobNode {
            name_idx: 1,
            pos: [0.0; 3],
            quat: [0.0, 0.0, 0.0, 1.0],
            next_sibling: NONE,
            first_child: NONE,
        }],
    ));
    assert!(!parse_head_nodes(&file).unwrap().has_hierarchy());
    assert!(parse_head_nodes(b"NOPE").is_err());
}
