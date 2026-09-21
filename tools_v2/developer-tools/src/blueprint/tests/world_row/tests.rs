use super::*;
use crate::blueprint::tests::fixture;
use crate::blueprint::verify::{load, verify};
use crate::repository_layout::terrain_dir;

fn objects_dir() -> std::path::PathBuf {
    let root = crate::repository_paths::find_repo_root().unwrap();
    terrain_dir(&root, "everon").join("objects")
}

#[test]
fn pid_lookup_ignores_the_guid_prefix() {
    let names: HashMap<u64, String> = [
        (3, "{AAAA}Prefabs/A/x.et".to_string()),
        (7, "{BBBB}Prefabs/B/y.et".to_string()),
    ]
    .into_iter()
    .collect();
    assert_eq!(pid_for_resource(&names, "Prefabs/B/y.et"), Some(7));
    assert_eq!(pid_for_resource(&names, "{CCCC}Prefabs/A/x.et"), Some(3));
    assert_eq!(pid_for_resource(&names, "Prefabs/C/z.et"), None);
}

#[test]
fn chunk_id_is_the_floor_partition() {
    assert_eq!(chunk_id_of(9363.58, 285.598), "18_0");
    assert_eq!(chunk_id_of(512.0, 511.999), "1_0");
    assert_eq!(chunk_id_of(0.0, 0.0), "0_0");
}

/// The transform pin: the farmhouse's committed chunk row (18_0), composed with
/// the 88 socket instances of the pipeline, lands every child on the Workbench
/// recon's absolute `worldPos` within 2 cm. Its pitch and roll are 0, so the row must have
/// stayed 5-wide (trivial trailers are never padded).
#[test]
fn farmhouse_chunk_row_places_every_socket_child_within_2cm() {
    let root = crate::repository_paths::test_repo_root();
    let instances =
        terrain_dir(&root, "everon").join("prefabs/buildings/FarmHouse_E_1L01_Wood.instances.json");
    let recon = fixture("FarmHouse_E_1L01_Wood_children.json");
    let (file, dump) = load(&instances, &recon).unwrap();
    let matches = verify(&file, &dump);
    assert_eq!(matches.matches.len(), 88);
    let names = load_prefab_names(&objects_dir().join("prefabs.json.gz")).unwrap();
    let pid = pid_for_resource(&names, &file.resource_name).expect("farmhouse pid");
    let rwp = dump.root_world_pos.expect("recon rootWorldPos");
    let id = chunk_id_of(rwp[0], rwp[2]);
    assert_eq!(id, "18_0");
    let rows = load_rows(&objects_dir().join(format!("chunks/{id}.json.gz"))).unwrap();
    let near = rows_near(&rows, pid, rwp[0], rwp[2], 0.05);
    assert_eq!(near.len(), 1, "{near:?}");
    let row = near[0];
    assert_eq!(row.width, 5, "pitch/roll 0 → the row stays 5-wide: {row:?}");
    assert_eq!(row.yaw, 38.46, "{row:?}");
    assert_eq!((row.pitch, row.roll, row.scale), (0.0, 0.0, 1.0));
    let r = check(row, &file, &dump, &matches);
    assert_eq!(r.checked, 88, "{r:?}");
    assert_eq!(r.children_without_world_pos, 0);
    assert!(
        r.worst_pos_m <= POS_TOL_M,
        "worst {:.4} m: {:?}",
        r.worst_pos_m,
        r.mismatches
    );
    assert!(r.ok());
}

/// The tilted-prop pin: the GarbageContainer_01 the rotation fixture recorded at
/// world (9878.51, 6.753, 236.2) with angles (−3.044, −104.126, −4.754) is an 8-wide row in
/// chunk 19_0 carrying pitch −3.04 / roll −4.75 (round2) and heading 255.87
/// (`norm_heading(−104.126)`). Scale is 1.0 while the catalogue is built from the July 2026
/// export (no `scale` field) — re-blessed when the v2 export lands.
#[test]
fn garbage_container_row_carries_pitch_and_roll() {
    let fx: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture("rotation_pin_GarbageContainer_01.json")).unwrap(),
    )
    .unwrap();
    let wp: Vec<f64> = fx["parent"]["worldPos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let ang: Vec<f64> = fx["parent"]["anglesDeg"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(ang, vec![-3.044, -104.126, -4.754]);
    let names = load_prefab_names(&objects_dir().join("prefabs.json.gz")).unwrap();
    let pid = pid_for_resource(&names, fx["parent"]["prefab"].as_str().unwrap())
        .expect("garbage container pid");
    let id = chunk_id_of(wp[0], wp[2]);
    assert_eq!(id, "19_0");
    let rows = load_rows(&objects_dir().join(format!("chunks/{id}.json.gz"))).unwrap();
    let near = rows_near(&rows, pid, wp[0], wp[2], 0.05);
    assert_eq!(near.len(), 1, "{near:?}");
    let row = near[0];
    assert_eq!(row.width, 8, "{row:?}");
    assert_eq!(row.z_up, 6.75);
    assert_eq!(row.yaw, 255.87);
    assert_eq!(row.pitch, -3.04);
    assert_eq!(row.roll, -4.75);
    assert_eq!(
        row.scale, 1.0,
        "scale absent from the July export (re-bless at 1b)"
    );
    // The engine-frame placement puts the origin where the fixture says it is (5 mm rounding).
    let t = row.rigid().point([0.0; 3]);
    assert!(
        (t[0] - wp[0]).abs() < 0.006
            && (t[1] - wp[1]).abs() < 0.006
            && (t[2] - wp[2]).abs() < 0.006,
        "{t:?}"
    );
}
