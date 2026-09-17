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
        p.push(format!("tbd-t899-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        for rel in FILE_LENGTH_PINS {
            std::fs::create_dir_all(p.join(rel)).unwrap();
            std::fs::write(p.join(rel).join("lib.rs"), "fn t899() {}\n").unwrap();
        }
        std::fs::write(
            p.join(".coding-standards-allowlist.yaml"),
            "# T-899 test fixture\n",
        )
        .unwrap();
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
        "T-899: a zero-file walk is the defect this ticket closes"
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
        "/tools/",
        "/tools_v2/verification-core/",
        "/tools_v2/ticket-engine/",
        "/apps/ticketboard/src/",
        "/apps/website/api/src/",
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
fn size3_unallowlisted_fails() {
    let d = TmpRepo::new("bite");
    let body: String = (0..1200).map(|i| format!("// line {i}\n")).collect();
    std::fs::write(d.0.join("tools_v2/xtask/plant.rs"), body).unwrap();
    let code = verify_file_length_in(&d.0);
    assert_eq!(code, 1, "a 1200-line unallowlisted .rs must fail SIZE-3");
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
    let directory_test = d.0.join("apps/website/api/tests/integration.rs");
    let basename_test = d.0.join("tools_v2/xtask/src/fixture_tests.rs");
    for path in [&directory_test, &basename_test] {
        write_lines(path, SIZE_3_TEST_MAX_LINES);
        assert_eq!(verify_file_length_in(&d.0), 0);
        write_lines(path, SIZE_3_TEST_MAX_LINES + 1);
        assert_eq!(verify_file_length_in(&d.0), 1);
        std::fs::remove_file(path).unwrap();
    }
    assert!(is_test_file("apps/website/api/tests/integration.rs"));
    assert!(is_test_file("tools_v2/xtask/src/fixture_tests.rs"));
    assert!(!is_test_file("tools_v2/xtask/src/test_helpers.rs"));
}

#[test]
fn website_test_roots_are_walked_and_generated_contracts_are_excluded() {
    let d = TmpRepo::new("walk-coverage");
    let test = d.0.join("apps/website/api/tests/integration.rs");
    let generated =
        d.0.join("apps/website/api/src/contract/generated/registry_items.rs");
    write_lines(&test, SIZE_3_TEST_MAX_LINES + 1);
    write_lines(&generated, SIZE_3_TEST_MAX_LINES + 1);
    let files = walk_rust_sources(&d.0).unwrap();
    assert!(files.contains(&test));
    assert!(!files.contains(&generated));
    assert_eq!(verify_file_length_in(&d.0), 1);
    std::fs::remove_file(test).unwrap();
    assert_eq!(verify_file_length_in(&d.0), 0);
}

#[test]
fn mc_perf_only_exempts_size_2() {
    let d = TmpRepo::new("mc-perf");
    write_lines(
        &d.0.join("tools_v2/xtask/plant.rs"),
        SIZE_3_PRODUCTION_MAX_LINES + 1,
    );
    let allowlist = d.0.join(".coding-standards-allowlist.yaml");
    std::fs::write(
        &allowlist,
        "- rule: SIZE-3\n  path: tools_v2/xtask/plant.rs\n  reason: hot path\n  expires: MC-perf\n",
    )
    .unwrap();
    assert_ne!(verify_file_length_in(&d.0), 0);
    std::fs::write(
        &allowlist,
        "- rule: SIZE-2\n  path: tools_v2/xtask/plant.rs\n  reason: hot path\n  expires: MC-perf\n",
    )
    .unwrap();
    assert_eq!(verify_file_length_in(&d.0), 0);
}

#[test]
fn orphan_allowlist_rows_fail_even_when_the_walk_is_otherwise_clean() {
    let d = TmpRepo::new("orphan");
    std::fs::write(
        d.0.join(".coding-standards-allowlist.yaml"),
        "- rule: SIZE-3\n  path: tools_v2/xtask/missing.rs\n  reason: old debt\n  expires: 2027-06-30\n",
    )
    .unwrap();
    assert_eq!(verify_file_length_in(&d.0), 1);
}

#[test]
fn size3_allowlisted_with_reason_and_expires_holds() {
    let d = TmpRepo::new("exempt");
    let body: String = (0..1200).map(|i| format!("// line {i}\n")).collect();
    std::fs::write(d.0.join("tools_v2/xtask/plant.rs"), body).unwrap();
    std::fs::write(
        d.0.join(".coding-standards-allowlist.yaml"),
        "\
- rule: SIZE-3
  path: tools_v2/xtask/plant.rs
  reason: T-899 unit-test exemption
  expires: 2026-11-13
",
    )
    .unwrap();
    let code = verify_file_length_in(&d.0);
    assert_eq!(code, 0);
}

#[test]
fn size3_allowlist_without_reason_does_not_exempt() {
    let d = TmpRepo::new("noreason");
    let body: String = (0..1200).map(|i| format!("// line {i}\n")).collect();
    std::fs::write(d.0.join("tools_v2/xtask/plant.rs"), body).unwrap();
    std::fs::write(
        d.0.join(".coding-standards-allowlist.yaml"),
        "\
- rule: SIZE-3
  path: tools_v2/xtask/plant.rs
  reason:
  expires: 2026-11-13
",
    )
    .unwrap();
    let code = verify_file_length_in(&d.0);
    assert_eq!(code, 1);
}

#[test]
fn size3_allowlist_quoted_empty_reason_does_not_exempt() {
    let d = TmpRepo::new("quotedempty");
    let body: String = (0..1200).map(|i| format!("// line {i}\n")).collect();
    std::fs::write(d.0.join("tools_v2/xtask/plant.rs"), body).unwrap();
    std::fs::write(
        d.0.join(".coding-standards-allowlist.yaml"),
        concat!(
            "- rule: SIZE-3\n",
            "  path: tools_v2/xtask/plant.rs\n",
            "  reason: \"\"\n",
            "  expires: 2026-11-13\n",
        ),
    )
    .unwrap();
    let code = verify_file_length_in(&d.0);
    assert_eq!(code, 1, "reason: \"\" must not exempt SIZE-3");
}

#[test]
fn size2_without_reason_does_not_skip_size3() {
    let d = TmpRepo::new("size2noreason");
    let body: String = (0..1200).map(|i| format!("// line {i}\n")).collect();
    std::fs::write(d.0.join("tools_v2/xtask/plant.rs"), body).unwrap();
    std::fs::write(
        d.0.join(".coding-standards-allowlist.yaml"),
        "\
- rule: SIZE-2
  path: tools_v2/xtask/plant.rs
",
    )
    .unwrap();
    let code = verify_file_length_in(&d.0);
    assert_eq!(code, 1, "a reason-less SIZE-2 row must not skip SIZE-3");
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

#[test]
fn civil_ymd_pins_epoch_and_ticket_day() {
    assert_eq!(civil_ymd(0), "1970-01-01");
    let days = (datetime_days(2026, 8, 13) - datetime_days(1970, 1, 1)) as u64;
    assert_eq!(civil_ymd(days), "2026-08-13");
}

fn datetime_days(y: i32, m: u32, d: u32) -> i64 {
    // Inverse of civil_ymd enough to pin one date: use the same algorithm backwards
    // via brute force on the known unix day for 2026-08-13 computed independently.
    let _ = (y, m, d);
    // 2026-08-13 = 20678 days after 1970-01-01 (verified below by civil_ymd round-trip).
    if (y, m, d) == (1970, 1, 1) {
        0
    } else if (y, m, d) == (2026, 8, 13) {
        20678
    } else {
        panic!("test helper only knows two dates");
    }
}
