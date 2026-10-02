use super::*;

#[test]
fn compiler_fixtures_resolve_from_root_crate_and_source_directory() {
    let root = find_repo_root().expect("active checkout");
    for relative in [
        "",
        "tools/developer_tools",
        "tools/developer_tools/src/blueprint",
    ] {
        let found = find_from(root.join(relative)).expect("repository root");
        assert_eq!(found, root);
        for fixture in [
            "FarmHouse_E_1L01_Wood.bvh.golden",
            "FarmHouse_E_1L01_Wood_children.json",
            "prefab/Prefabs/Houses/House_Wood.et",
        ] {
            assert!(
                found
                    .join("tools/developer_tools/test_fixtures/blueprint")
                    .join(fixture)
                    .is_file()
            );
        }
    }
}

#[test]
fn missing_repository_marker_is_an_error() {
    assert!(find_from(PathBuf::from("/")).is_err());
}
