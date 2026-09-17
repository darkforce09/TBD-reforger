use super::*;

/// T-912.1: the hardcoded dependency/run-last tables must not come back — the packer reads
/// ticket `depends_on` / `pack_last` / `owns`. The needle is assembled at runtime so this
/// test's own source cannot satisfy the scan it performs.
#[test]
fn hardcoded_dep_tables_stay_deleted() {
    let src = include_str!("../../collisions.rs");
    for name in ["DEPS", "RUN_LAST"] {
        let needle = format!("const {name}");
        assert!(
            !src.contains(&needle),
            "`{needle}` is back in slice_collisions.rs — T-912.1 moved it onto the tickets"
        );
    }
}

/// The packing facts come from the ticket files, children included — the parents-only
/// registry loader would return no owns for any mod-plan row.
#[test]
fn facts_come_from_ticket_files() {
    let facts = ticket_facts(&worktree_root()).expect("ticket facts");
    assert!(facts.pack_last.contains("T-290"), "T-290 lost pack_last");
    assert_eq!(facts.deps_of("T-212").join(","), "T-685,T-241,T-257");
    assert_eq!(facts.deps_of("T-238").join(","), "T-273,T-237");
    assert_eq!(facts.deps_of("T-251").join(","), "T-209");
    assert!(
        facts.owns.contains_key("T-181.23"),
        "child ticket owns must be globbed, not read through the parents-only loader"
    );
}
