use super::*;

#[test]
fn cksum_matches_the_coreutils_tool() {
    // If this drifts, the bash gate and this one fight over target/gate-schema and each pays a
    // cold rebuild. Compared against the real `cksum` so the interop claim is measured.
    use std::io::Write;
    let data = b"the quick brown fox\n";
    let mut child = std::process::Command::new("cksum")
        .current_dir(std::env::temp_dir())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("cksum on PATH");
    child.stdin.take().unwrap().write_all(data).unwrap();
    let out = child.wait_with_output().unwrap();
    let want: String = String::from_utf8_lossy(&out.stdout)
        .trim_end()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    assert_eq!(cksum(data), want);
}

#[test]
fn empty_input_matches_too() {
    assert_eq!(cksum(b""), format!("{}{}", 4294967295u32, 0));
}

/// A read that returns only some of the names leaves a one-way subset check green over the
/// hole, so the set must match EXACTLY — an empty or partial read is a hard fail in
/// `gate_schema`.
///
/// Both sides come from code: the task table and the pinned constant. No `if …exists()` guard
/// skips the comparison, because a guard makes the test vacuous the moment its subject goes away.
#[test]
fn the_task_table_and_the_pinned_set_agree() {
    let mut got = task_validate_gates();
    assert!(
        !got.is_empty(),
        "the schema-validate task row vanished — gate_schema would refuse, and so does this"
    );
    got.sort();
    let mut want: Vec<String> = VALIDATE_GATES.iter().map(|s| (*s).to_string()).collect();
    want.sort();
    assert_eq!(
        got, want,
        "`xtask schema list-gates` disagrees with GATE_SCHEMA_VALIDATE_GATES"
    );
}

/// The stamp roots are xtask's build closure, derived from the manifests: xtask itself, its
/// direct path dependencies, the members only reachable through them, and nothing it does not
/// build. A hand list missed every crate born after it was written, and two trees could then
/// share the private target under one stamp while a dependency differed.
#[test]
fn the_stamp_roots_are_the_whole_build_closure_of_xtask_and_nothing_else() {
    let root = tool_test_support::test_repo_root();
    let roots = xtask_build_closure(&root).expect("the workspace manifests read");
    for wanted in [
        "tools/xtask",
        "tools/commands/platform_execution",
        "tools/commands/ci_task_catalog",
        "tools/tickets/ticket_registry",
        "tools/foundation/process_runner",
        // Reached only through other members: developer_tools links the map engine, the
        // ticket model links the id macros.
        "legacy/map_engine",
        "crates/foundation/newtype_ids",
    ] {
        assert!(
            roots.iter().any(|folder| folder == wanted),
            "{wanted} is missing from the stamp roots {roots:?}"
        );
    }
    for unwanted in [
        "apps/api",
        "apps/frontend",
        "tools/foundation/tool_test_support",
    ] {
        assert!(
            !roots.iter().any(|folder| folder == unwanted),
            "{unwanted} is not built with xtask, yet it is a stamp root"
        );
    }
    let mut sorted = roots.clone();
    sorted.sort();
    assert_eq!(
        roots, sorted,
        "the stamp roots are listed in a stable order"
    );
}
