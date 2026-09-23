use super::*;
use std::os::unix::fs::PermissionsExt;

fn this_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tools_v2/xtask has a parent")
        .parent()
        .expect("xtask is not the repo root")
        .to_path_buf()
}

struct TmpRepo(PathBuf);
impl TmpRepo {
    fn new(name: &str) -> TmpRepo {
        let mut p = std::env::temp_dir();
        p.push(format!("xtask-file-length-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        for rel in FILE_LENGTH_PINS {
            std::fs::create_dir_all(p.join(rel)).unwrap();
            std::fs::write(p.join(rel).join("lib.rs"), "fn placeholder() {}\n").unwrap();
        }
        TmpRepo(p)
    }
}
impl Drop for TmpRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn walk_is_nonempty_anti_vacuity() {
    let files = walk_rust_sources(&this_repo()).expect("walk must run");
    assert!(
        !files.is_empty(),
        "a zero-file walk must never read as a pass"
    );
    assert!(
        files
            .iter()
            .all(|p| p.extension().and_then(|e| e.to_str()) == Some("rs"))
    );
    let joined = files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n");
    for needle in [
        "/tools_v2/xtask/",
        "/tools_v2/developer-tools/",
        "/tools_v2/verification-core/",
        "/tools_v2/ticket-engine/",
        "/apps/ticketboard/src/",
        "/apps/website/api_v2/src/",
        "/apps/website/frontend/src/",
    ] {
        assert!(
            joined.contains(needle),
            "walk missed pin {needle}; first files: {:?}",
            files.iter().take(5).collect::<Vec<_>>()
        );
    }
}

#[test]
fn missing_walk_root_is_did_not_run() {
    for missing in FILE_LENGTH_PINS {
        let d = TmpRepo::new("missing");
        std::fs::remove_dir_all(d.0.join(missing)).unwrap();
        let code = verify_file_length_in(&d.0);
        assert_eq!(code, 2, "missing {missing} must not read as 0/0");
    }
}

#[test]
fn unreadable_file_is_did_not_run() {
    let d = TmpRepo::new("unreadable");
    let f = d.0.join("tools_v2/xtask/secret.rs");
    std::fs::write(&f, "fn x() {}\n").unwrap();
    let mut perms = std::fs::metadata(&f).unwrap().permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&f, perms).unwrap();
    let code = verify_file_length_in(&d.0);
    let mut perms = std::fs::metadata(&f).unwrap().permissions();
    perms.set_mode(0o644);
    let _ = std::fs::set_permissions(&f, perms);
    assert_eq!(code, 2, "an unreadable .rs must not count as 0 lines");
}

#[test]
fn size3_exceeding_fails() {
    let d = TmpRepo::new("bite");
    let body: String = (0..1200).map(|i| format!("// line {i}\n")).collect();
    std::fs::write(d.0.join("tools_v2/xtask/plant.rs"), body).unwrap();
    let code = verify_file_length_in(&d.0);
    assert_eq!(code, 1, "a 1200-line .rs must fail SIZE-3");
}

fn write_lines(path: &Path, count: usize) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "// line\n".repeat(count)).unwrap();
}

#[test]
fn production_boundary_is_500_lines() {
    let d = TmpRepo::new("production-boundary");
    let path = d.0.join("apps/ticketboard/src/board.rs");
    write_lines(&path, SIZE_3_PRODUCTION_MAX_LINES);
    assert_eq!(verify_file_length_in(&d.0), 0);
    write_lines(&path, SIZE_3_PRODUCTION_MAX_LINES + 1);
    assert_eq!(verify_file_length_in(&d.0), 1);
}

#[test]
fn test_boundary_is_1000_lines_for_directory_and_basename() {
    let d = TmpRepo::new("test-boundary");
    let directory_test = d.0.join("apps/website/api_v2/tests/integration.rs");
    let basename_test = d.0.join("tools_v2/xtask/src/fixture_tests.rs");
    for path in [&directory_test, &basename_test] {
        write_lines(path, SIZE_3_TEST_MAX_LINES);
        assert_eq!(verify_file_length_in(&d.0), 0);
        write_lines(path, SIZE_3_TEST_MAX_LINES + 1);
        assert_eq!(verify_file_length_in(&d.0), 1);
        std::fs::remove_file(path).unwrap();
    }
    assert!(is_test_file("apps/website/api_v2/tests/integration.rs"));
    assert!(is_test_file("tools_v2/xtask/src/fixture_tests.rs"));
    assert!(!is_test_file("tools_v2/xtask/src/test_helpers.rs"));
}

#[test]
fn website_test_roots_and_generated_contracts_are_walked_without_exemption() {
    let d = TmpRepo::new("walk-coverage");
    let test = d.0.join("apps/website/api_v2/tests/integration.rs");
    let generated = [
        "apps/website/api_v2/src/missions/contract/generated/registry_items/mod.rs",
        "apps/website/api_v2/src/missions/models/generated/mission_review/mission_row.rs",
        "apps/website/api_v2/src/operations/models/generated/event_hub/mod.rs",
        "apps/website/api_v2/src/server_infrastructure/models/generated/fleet_command/error.rs",
    ]
    .map(|path| d.0.join(path));
    write_lines(&test, SIZE_3_TEST_MAX_LINES + 1);
    for path in &generated {
        write_lines(path, SIZE_3_PRODUCTION_MAX_LINES + 1);
    }
    let files = walk_rust_sources(&d.0).unwrap();
    assert!(files.contains(&test));
    for path in &generated {
        assert!(files.contains(path), "{}", path.display());
    }
    assert_eq!(verify_file_length_in(&d.0), 1);
    std::fs::remove_file(test).unwrap();
    for path in &generated {
        assert_eq!(
            verify_file_length_in(&d.0),
            1,
            "generated code is held to the limit too: {}",
            path.display()
        );
        std::fs::remove_file(path).unwrap();
    }
    assert_eq!(verify_file_length_in(&d.0), 0);
}

#[test]
fn allowlist_file_must_not_exist() {
    let root = this_repo();
    assert!(
        !root.join(".coding-standards-allowlist.yaml").exists(),
        ".coding-standards-allowlist.yaml must remain permanently deleted"
    );
}

#[test]
fn size3_has_zero_exemptions_even_if_allowlist_is_attempted() {
    let d = TmpRepo::new("no-exemptions");
    let body: String = (0..1200).map(|i| format!("// line {i}\n")).collect();
    std::fs::write(d.0.join("tools_v2/xtask/plant.rs"), body).unwrap();
    std::fs::write(
        d.0.join(".coding-standards-allowlist.yaml"),
        "- rule: SIZE-3\n  path: tools_v2/xtask/plant.rs\n  reason: attempt\n  expires: 2030-01-01\n",
    )
    .unwrap();
    let code = verify_file_length_in(&d.0);
    assert_eq!(code, 1, "SIZE-3 must reject any attempts to exempt files");
}

#[test]
fn empty_walk_is_not_ok() {
    let d = TmpRepo::new("vacuous");
    for rel in FILE_LENGTH_PINS {
        std::fs::remove_file(d.0.join(rel).join("lib.rs")).unwrap();
    }
    let code = verify_file_length_in(&d.0);
    assert_ne!(code, 0, "zero .rs files must not print 0/0 OK");
}
