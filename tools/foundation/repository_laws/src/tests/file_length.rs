//! Tests for [`super`] — the 500 and 1000 line ceilings, their report lines, and the refusals.

use std::os::unix::fs::PermissionsExt;

use super::*;
use crate::temporary_checkout::{FIXTURE_WORKSPACE_MEMBERS, TemporaryCheckout};

fn violations(checkout: &TemporaryCheckout) -> Vec<FileLengthViolation> {
    scan_file_lengths(checkout.root()).unwrap().violations
}

#[test]
fn a_production_file_may_hold_exactly_500_lines() {
    let checkout = TemporaryCheckout::with_law_roots("production-ceiling");
    checkout.write_lines(
        "tools/tickets/ticketboard_desktop/src/board.rs",
        PRODUCTION_MAX_LINES,
    );
    assert!(violations(&checkout).is_empty());
    checkout.write_lines(
        "tools/tickets/ticketboard_desktop/src/board.rs",
        PRODUCTION_MAX_LINES + 1,
    );
    assert_eq!(
        violations(&checkout),
        [FileLengthViolation {
            path: "tools/tickets/ticketboard_desktop/src/board.rs".into(),
            lines: 501,
            max_lines: 500,
        }]
    );
}

#[test]
fn a_test_file_may_hold_exactly_1000_lines_by_folder_or_by_stem() {
    let checkout = TemporaryCheckout::with_law_roots("test-ceiling");
    for rel in [
        "apps/server/tests/integration.rs",
        "tools/xtask/src/fixture_tests.rs",
        "tools/xtask/fixtures/TBD_ScriptPlant_tests.c",
    ] {
        checkout.write_lines(rel, TEST_MAX_LINES);
        assert!(violations(&checkout).is_empty(), "{rel}");
        checkout.write_lines(rel, TEST_MAX_LINES + 1);
        let found = violations(&checkout);
        assert_eq!(found.len(), 1, "{rel}");
        assert!(found[0].is_test_file(), "{rel}");
        std::fs::remove_file(checkout.root().join(rel)).unwrap();
    }
}

#[test]
fn an_enfusion_script_meets_the_production_ceiling() {
    let checkout = TemporaryCheckout::with_law_roots("script-ceiling");
    checkout.write_lines(
        "tools/xtask/fixtures/TBD_ScriptPlant.c",
        PRODUCTION_MAX_LINES,
    );
    assert!(violations(&checkout).is_empty());
    checkout.write_lines(
        "tools/xtask/fixtures/TBD_ScriptPlant.c",
        PRODUCTION_MAX_LINES + 1,
    );
    assert_eq!(violations(&checkout).len(), 1);
}

#[test]
fn generated_code_and_website_test_trees_have_no_exemption() {
    let checkout = TemporaryCheckout::with_law_roots("no-exemption");
    checkout.write_lines(
        "crates/contracts/contract_schema_types/src/generated/missions/mission_review/approval_queue_row.rs",
        PRODUCTION_MAX_LINES + 1,
    );
    checkout.write_lines(
        "crates/geometry/camera_math/tests/cases.rs",
        TEST_MAX_LINES + 1,
    );
    checkout.write(
        ".coding-standards-allowlist.yaml",
        "- path: crates/geometry/camera_math/tests/cases.rs\n",
    );
    assert_eq!(violations(&checkout).len(), 2);
}

#[test]
fn the_report_lines_are_the_gate_contract() {
    let checkout = TemporaryCheckout::with_law_roots("report-lines");
    checkout.write_lines("tools/xtask/fixtures/TBD_ScriptPlant.c", 3);
    checkout.write_lines("tools/xtask/plant.rs", 1200);
    let scan = scan_file_lengths(checkout.root()).unwrap();
    assert_eq!(
        scan.violations[0].rendered(),
        "SIZE-3: tools/xtask/plant.rs is 1200 lines (>500). Hard limit exceeded; decompose by \
         responsibility. No exemptions permitted."
    );
    let rust = FIXTURE_WORKSPACE_MEMBERS.len() + 1;
    assert_eq!(
        scan.summary(),
        format!(
            "file-length: scanned {} source file(s) ({rust} .rs, 1 .c), 1 violation(s).",
            rust + 1
        )
    );
}

#[test]
fn an_unreadable_file_is_a_scan_that_did_not_run() {
    let checkout = TemporaryCheckout::with_law_roots("unreadable");
    checkout.write("tools/xtask/secret.rs", "fn x() {}\n");
    let path = checkout.root().join("tools/xtask/secret.rs");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    let outcome = scan_file_lengths(checkout.root());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(outcome, Err(NotRun::Unreadable { .. })));
}

#[test]
fn an_empty_walk_returns_no_files_for_the_caller_to_refuse() {
    let checkout = TemporaryCheckout::with_law_roots("empty");
    for member in FIXTURE_WORKSPACE_MEMBERS {
        std::fs::remove_file(checkout.root().join(member).join("src/lib.rs")).unwrap();
    }
    let scan = scan_file_lengths(checkout.root()).unwrap();
    assert!(scan.files.is_empty() && scan.violations.is_empty());
}
