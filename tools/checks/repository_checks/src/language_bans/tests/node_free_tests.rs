use super::*;
use std::os::unix::fs::PermissionsExt;

fn this_repo() -> PathBuf {
    repository_root::find_repository_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("repository root")
}

/// The member folders a [`TmpRepo`] workspace declares: the folders the repository's own members
/// sit in.
const FIXTURE_WORKSPACE_MEMBERS: &[&str] = &[
    "apps/fleet_host_agent",
    "apps/ticketboard",
    "apps/api",
    "apps/frontend",
    "crates/geometry/camera_math",
    "crates/graphics/render_primitives",
    "apps/offline_service_worker",
    "tools/foundation/verification_core",
    "tools/foundation/process_runner",
    "tools/foundation/repository_laws",
    "tools/tickets/ticket_model",
    "tools/xtask",
    "tools/developer_tools",
    "tools/foundation/repository_layout",
    "crates/contracts/contract_schema_types",
];

/// A temporary checkout holding every law root: a workspace of [`FIXTURE_WORKSPACE_MEMBERS`],
/// each with a `Cargo.toml` and a one-line `src/lib.rs`, and every pinned script root, empty.
struct TmpRepo(PathBuf);
impl TmpRepo {
    fn new(name: &str) -> TmpRepo {
        let mut p = std::env::temp_dir();
        p.push(format!("xtask-file-length-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        let listed: String = FIXTURE_WORKSPACE_MEMBERS
            .iter()
            .map(|member| format!("    \"{member}\",\n"))
            .collect();
        std::fs::write(
            p.join("Cargo.toml"),
            format!("[workspace]\nmembers = [\n{listed}]\n"),
        )
        .unwrap();
        for member in FIXTURE_WORKSPACE_MEMBERS {
            let package = member.rsplit('/').next().unwrap_or(member);
            std::fs::create_dir_all(p.join(member).join("src")).unwrap();
            std::fs::write(
                p.join(member).join("Cargo.toml"),
                format!("[package]\nname = \"{package}\"\n"),
            )
            .unwrap();
            std::fs::write(p.join(member).join("src/lib.rs"), "fn placeholder() {}\n").unwrap();
        }
        for root in PINNED_SCRIPT_ROOTS {
            std::fs::create_dir_all(p.join(root)).unwrap();
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
    let files = walk_length_gated_sources(&this_repo()).expect("walk must run");
    assert!(
        !files.is_empty(),
        "a zero-file walk must never read as a pass"
    );
    assert!(
        files
            .iter()
            .all(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("rs" | "c")))
    );
    let joined = files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n");
    for needle in [
        "/tools/xtask/",
        "/tools/developer_tools/",
        "/tools/foundation/verification_core/",
        "/tools/foundation/process_runner/",
        "/tools/foundation/repository_laws/",
        "/tools/tickets/ticket_model/",
        "/apps/ticketboard/src/",
        "/apps/api/src/",
        "/apps/frontend/src/",
        "/apps/mod/tbd-framework/Scripts/",
        "/apps/mod/tbd-emcp/Scripts/",
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
    for missing in FIXTURE_WORKSPACE_MEMBERS.iter().chain(PINNED_SCRIPT_ROOTS) {
        let d = TmpRepo::new("missing");
        std::fs::remove_dir_all(d.0.join(missing)).unwrap();
        let code = verify_file_length_in(&d.0);
        assert_eq!(code, 2, "missing {missing} must not read as 0/0");
    }
}

#[test]
fn unreadable_file_is_did_not_run() {
    let d = TmpRepo::new("unreadable");
    let f = d.0.join("tools/xtask/secret.rs");
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
    std::fs::write(d.0.join("tools/xtask/plant.rs"), body).unwrap();
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
    let directory_test = d.0.join("apps/api/tests/integration.rs");
    let basename_test = d.0.join("tools/xtask/src/fixture_tests.rs");
    for path in [&directory_test, &basename_test] {
        write_lines(path, SIZE_3_TEST_MAX_LINES);
        assert_eq!(verify_file_length_in(&d.0), 0);
        write_lines(path, SIZE_3_TEST_MAX_LINES + 1);
        assert_eq!(verify_file_length_in(&d.0), 1);
        std::fs::remove_file(path).unwrap();
    }
    assert!(is_test_file("apps/api/tests/integration.rs"));
    assert!(is_test_file("tools/xtask/src/fixture_tests.rs"));
    assert!(!is_test_file("tools/xtask/src/test_helpers.rs"));
}

#[test]
fn enfusion_script_production_boundary_is_500_lines() {
    let d = TmpRepo::new("script-production-boundary");
    let path = d.0.join("tools/xtask/fixtures/TBD_ScriptPlant.c");
    write_lines(&path, SIZE_3_PRODUCTION_MAX_LINES);
    assert_eq!(verify_file_length_in(&d.0), 0, "a 500-line .c passes");
    write_lines(&path, SIZE_3_PRODUCTION_MAX_LINES + 1);
    assert_eq!(verify_file_length_in(&d.0), 1, "a 501-line .c fails");
}

#[test]
fn enfusion_script_tests_basename_holds_to_1000_lines() {
    let d = TmpRepo::new("script-test-boundary");
    let path = d.0.join("tools/xtask/fixtures/TBD_ScriptPlant_tests.c");
    write_lines(&path, SIZE_3_TEST_MAX_LINES);
    assert_eq!(
        verify_file_length_in(&d.0),
        0,
        "a 1000-line *_tests.c passes"
    );
    write_lines(&path, SIZE_3_TEST_MAX_LINES + 1);
    assert_eq!(
        verify_file_length_in(&d.0),
        1,
        "a 1001-line *_tests.c fails"
    );
    assert!(is_test_file(
        "apps/mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Hash_tests.c"
    ));
    assert!(!is_test_file(
        "apps/mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Hash.c"
    ));
    assert!(!is_test_file(
        "apps/mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Hash_tests.h"
    ));
}

#[test]
fn summary_reports_the_mixed_extension_count() {
    let d = TmpRepo::new("mixed-count");
    write_lines(&d.0.join("tools/xtask/fixtures/TBD_ScriptPlant.c"), 3);
    write_lines(&d.0.join("tools/xtask/fixtures/notes.txt"), 3);
    let files = walk_length_gated_sources(&d.0).unwrap();
    let rust_count = FIXTURE_WORKSPACE_MEMBERS.len();
    assert_eq!(files.len(), rust_count + 1, "the .txt is not walked");
    assert_eq!(
        length_scan_summary(&files, 0),
        format!(
            "file-length: scanned {} source file(s) ({rust_count} .rs, 1 .c), 0 violation(s).",
            rust_count + 1
        )
    );
}

#[test]
fn mod_script_roots_are_the_three_shipped_addons() {
    let root = this_repo();
    for script_root in MOD_SCRIPT_ROOTS {
        assert!(root.join(script_root).is_dir(), "{script_root} must exist");
    }
    for reference in ["crf_framework", "vanilla_reference"] {
        assert!(
            MOD_SCRIPT_ROOTS.iter().all(|r| !r.contains(reference)),
            "{reference} is a gitignored reference and is never gated"
        );
    }
    assert!(mod_pins_are_script_roots(
        &["tools/xtask", "apps/mod/tbd-emcp/Scripts"],
        MOD_SCRIPT_ROOTS
    ));
    assert!(!mod_pins_are_script_roots(&["apps/mod"], MOD_SCRIPT_ROOTS));
    assert!(!mod_pins_are_script_roots(
        &["apps/mod/vanilla_reference/Scripts"],
        MOD_SCRIPT_ROOTS
    ));
}

#[test]
fn website_test_roots_and_generated_contracts_are_walked_without_exemption() {
    let d = TmpRepo::new("walk-coverage");
    let test = d.0.join("apps/api/tests/integration.rs");
    let generated = [
        "crates/contracts/contract_schema_types/src/generated/missions/registry_items/mod.rs",
        "crates/contracts/contract_schema_types/src/generated/missions/mission_review/approval_queue_row.rs",
        "crates/contracts/contract_schema_types/src/generated/operations/event_hub/mod.rs",
        "crates/contracts/contract_schema_types/src/generated/server_infrastructure/fleet_command/claim_request.rs",
    ]
    .map(|path| d.0.join(path));
    write_lines(&test, SIZE_3_TEST_MAX_LINES + 1);
    for path in &generated {
        write_lines(path, SIZE_3_PRODUCTION_MAX_LINES + 1);
    }
    let files = walk_length_gated_sources(&d.0).unwrap();
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
    std::fs::write(d.0.join("tools/xtask/plant.rs"), body).unwrap();
    std::fs::write(
        d.0.join(".coding-standards-allowlist.yaml"),
        "- rule: SIZE-3\n  path: tools/xtask/plant.rs\n  reason: attempt\n  expires: 2030-01-01\n",
    )
    .unwrap();
    let code = verify_file_length_in(&d.0);
    assert_eq!(code, 1, "SIZE-3 must reject any attempts to exempt files");
}

#[test]
fn empty_walk_is_not_ok() {
    let d = TmpRepo::new("vacuous");
    for member in FIXTURE_WORKSPACE_MEMBERS {
        std::fs::remove_file(d.0.join(member).join("src/lib.rs")).unwrap();
    }
    let code = verify_file_length_in(&d.0);
    assert_ne!(code, 0, "zero .rs files must not print 0/0 OK");
}
