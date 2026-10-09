use super::*;

#[test]
fn pid_lookup_ignores_the_guid_prefix() {
    let names: HashMap<u64, String> = [
        (3, "{AAAA}Prefabs/A/x.et".to_string()),
        (7, "{BBBB}Prefabs/B/y.et".to_string()),
    ]
    .into_iter()
    .collect();
    assert_eq!(prefab_id_for_resource(&names, "Prefabs/B/y.et"), Some(7));
    assert_eq!(
        prefab_id_for_resource(&names, "{CCCC}Prefabs/A/x.et"),
        Some(3)
    );
    assert_eq!(prefab_id_for_resource(&names, "Prefabs/C/z.et"), None);
}

#[test]
fn chunk_id_is_the_floor_partition() {
    assert_eq!(chunk_id_of(9363.58, 285.598), "18_0");
    assert_eq!(chunk_id_of(512.0, 511.999), "1_0");
    assert_eq!(chunk_id_of(0.0, 0.0), "0_0");
}
