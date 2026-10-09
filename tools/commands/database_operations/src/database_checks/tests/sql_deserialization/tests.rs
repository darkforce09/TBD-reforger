use super::*;

#[test]
fn extracts_the_table_name() {
    let p = Pattern::regex(r"SELECT \* FROM ([a-z_]+)").unwrap();
    assert_eq!(
        tables_in(&p, "  q(\"SELECT * FROM users WHERE id = $1\")"),
        vec!["users"]
    );
    assert_eq!(
        tables_in(&p, "SELECT * FROM modpack_mods; SELECT * FROM events"),
        vec!["modpack_mods", "events"]
    );
    assert!(tables_in(&p, "no match here").is_empty());
}

#[test]
fn allowlisted_tables_are_exempt() {
    assert!(ALLOW.contains(&"modpack_mods"));
    assert!(ALLOW.contains(&"orbat_reservations"));
    assert!(!ALLOW.contains(&"users"));
}

#[test]
fn a_missing_api_tree_does_not_read_as_clean() {
    // The bash `2>/dev/null || true` behaviour, inverted.
    let code = verify_no_select_star(Path::new("/nonexistent/verification/repo")).unwrap();
    assert_eq!(code, 2, "a scan that never ran must not exit 0");
}

#[test]
fn no_scan_root_lies_inside_another_so_no_package_is_read_twice() {
    for (index, root) in ROOTS.iter().enumerate() {
        for (other_index, other) in ROOTS.iter().enumerate() {
            assert!(
                index == other_index || !Path::new(root).starts_with(other),
                "the scan root {root} lies inside {other}: its files would be read twice"
            );
        }
    }
}

/// The live checkout: the walk reaches the API server's sources and every API source once.
#[test]
fn this_checkout_walks_every_api_source_once() {
    let repo_root = tool_test_support::test_repo_root();
    let files = api_source_files(&repo_root).expect("the API sources walk");
    let distinct: std::collections::BTreeSet<&PathBuf> = files.iter().collect();
    assert_eq!(
        distinct.len(),
        files.len(),
        "a file is walked more than once"
    );
    let server_sources = repo_root.join("crates/api/api_server/src");
    assert!(
        files.iter().any(|file| file.starts_with(&server_sources)),
        "the API server's sources are in scope"
    );
}
