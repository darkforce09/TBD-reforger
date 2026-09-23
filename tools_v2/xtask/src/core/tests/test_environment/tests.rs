//! These tests assert on the pure `prepended_path` and never write process `PATH`: sibling tests
//! spawn `git` and `grep` without `ENV_LOCK`, so a stub-only PATH here would break them.

use super::*;

const STUB_DIR: &str = "/tmp/path-guard-stub";

fn path_entries(path: &str) -> Vec<&str> {
    path.split(':').collect()
}

#[test]
fn prepended_path_onto_empty_path_is_dir_then_system_bins() {
    assert_eq!(
        prepended_path(STUB_DIR, ""),
        "/tmp/path-guard-stub:/usr/bin:/bin"
    );
}

#[test]
fn prepended_path_onto_stub_only_path_keeps_system_bins_reachable() {
    let old = "/tmp/only-stub-bin";
    let next = prepended_path(STUB_DIR, old);
    let entries = path_entries(&next);
    assert!(
        next.starts_with("/tmp/path-guard-stub:"),
        "PATH must search the prepended dir first: {next}"
    );
    assert!(
        entries.contains(&"/usr/bin"),
        "PATH must keep /usr/bin: {next}"
    );
    assert!(entries.contains(&"/bin"), "PATH must keep /bin: {next}");
    assert!(
        entries.contains(&old),
        "PATH must keep the old entry: {next}"
    );
}

#[test]
fn prepended_path_onto_path_with_system_bins_only_prepends_dir() {
    let old = "/home/operator/.cargo/bin:/usr/bin:/usr/local/bin";
    assert_eq!(
        prepended_path(STUB_DIR, old),
        format!("/tmp/path-guard-stub:{old}")
    );
}
