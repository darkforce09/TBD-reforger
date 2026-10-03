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
