use super::super::fixture_checkout::{FixtureCheckout, failures, outcome_counts};
use super::*;
use crate::core::repository_layout::documentation::{
    ARCHIVE_DIR, PENDING_MERGE_DIR, PROGRAM_RECORDS_PREFIX, TICKET_DOCUMENTS_DIR,
};

fn run(fixture: &FixtureCheckout, scope: &[&str]) -> GateRun {
    let scope: Vec<String> = scope.iter().map(ToString::to_string).collect();
    judge(fixture.root(), Ok(fixture.tree()), &scope)
}

fn lines(count: usize) -> String {
    "a line\n".repeat(count)
}

#[test]
fn code_trees_hold_only_readme_markdown() {
    let mut fixture = FixtureCheckout::new("placement-code");
    fixture
        .tracked("apps/README.md", "# Apps\n")
        .tracked("apps/tool/NOTES.md", "# Notes\n")
        .tracked("apps/tool/rules.MD", "# Rules\n")
        .tracked("apps/tool/tests/fixture.md", "")
        .tracked("apps/tool/generated/api.md", "")
        .tracked("apps/.cursor/guide.md", "")
        .tracked("apps/tool/main.rs", "");
    let run = run(&fixture, &[]);
    assert_eq!(
        failures(&run),
        [
            "FAIL: apps/tool/NOTES.md: Markdown in a code tree; a code tree holds only README.md, \
             and documents live under documentation_v2/",
            "FAIL: apps/tool/rules.MD: Markdown in a code tree; a code tree holds only README.md, \
             and documents live under documentation_v2/",
        ]
    );
    assert_eq!(
        outcome_counts(&run),
        (2, 2, 0),
        "README.md and the empty retired root held"
    );
    assert_eq!(
        run.totals[0],
        "  code trees: 3 Markdown file(s) judged, 2 other than README.md"
    );
    assert_eq!(run.print(), 1);
}

#[test]
fn the_retired_root_holds_no_tracked_file() {
    let mut fixture = FixtureCheckout::new("placement-retired");
    fixture
        .tracked(&format!("{RETIRED_DOCS_ROOT}/guide.md"), "# Guide\n")
        .tracked(&format!("{RETIRED_DOCS_ROOT}/images/map.png"), "")
        .tracked("apps/README.md", "# Apps\n");
    let run = run(&fixture, &[]);
    let root = RETIRED_DOCS_ROOT;
    assert_eq!(
        failures(&run),
        [format!(
            "FAIL: {root}/ holds 2 tracked file(s); every document lives under documentation_v2/\n      \
             {root}/guide.md\n      {root}/images/map.png"
        )]
    );
    assert_eq!(run.totals[1], format!("  {root}/: 2 tracked file(s)"));
}

#[test]
fn live_documents_stay_at_or_under_the_limit() {
    let mut fixture = FixtureCheckout::new("placement-size");
    fixture
        .tracked("documentation_v2/runbooks/short.md", &lines(500))
        .tracked("documentation_v2/runbooks/long.md", &lines(501))
        .tracked("documentation_v2/runbooks/table.csv", &lines(900));
    let run = run(&fixture, &[]);
    assert_eq!(
        failures(&run),
        [
            "FAIL: documentation_v2/runbooks/long.md: 501 lines; a live document stays at or under \
          500, so split it by topic into a folder with a README.md index"
        ]
    );
    assert_eq!(
        run.totals[2],
        "  documentation_v2/: 2 live document(s) judged, 1 over 500 lines, 0 unreadable"
    );
}

#[test]
fn frozen_pending_and_record_documents_are_outside_the_limit() {
    let mut fixture = FixtureCheckout::new("placement-exempt");
    let long = lines(900);
    for exempt in [
        format!("{TICKET_DOCUMENTS_DIR}/specs/t1_example.md"),
        format!("{ARCHIVE_DIR}/topic/old_plan.md"),
        format!("{PENDING_MERGE_DIR}/writer/source.md"),
        format!("{PROGRAM_RECORDS_PREFIX}program_plan.md"),
        format!("{PROGRAM_RECORDS_PREFIX}move_manifest/live_targets.md"),
    ] {
        fixture.tracked(&exempt, &long);
    }
    fixture.tracked("documentation_v2/README.md", &lines(10));
    let run = run(&fixture, &[]);
    assert_eq!(failures(&run), Vec::<String>::new());
    assert_eq!(
        outcome_counts(&run),
        (2, 0, 0),
        "the root README and the retired root"
    );
}

#[test]
fn a_document_missing_from_the_disk_did_not_run() {
    let mut fixture = FixtureCheckout::new("placement-unreadable");
    fixture.listed_only("documentation_v2/runbooks/gone.md");
    let run = run(&fixture, &[]);
    assert_eq!(outcome_counts(&run), (1, 0, 1));
    assert_eq!(
        run.totals[2],
        "  documentation_v2/: 1 live document(s) judged, 0 over 500 lines, 1 unreadable"
    );
    assert_eq!(run.print(), 2);
}

#[test]
fn the_scope_narrows_every_rule() {
    let mut fixture = FixtureCheckout::new("placement-scope");
    fixture
        .tracked("apps/tool/NOTES.md", "# Notes\n")
        .tracked(&format!("{RETIRED_DOCS_ROOT}/guide.md"), "# Guide\n")
        .tracked("documentation_v2/long.md", &lines(600))
        .tracked("documentation_v2/area/short.md", &lines(5));
    let documentation = run(&fixture, &["documentation_v2/area"]);
    assert_eq!(outcome_counts(&documentation), (1, 0, 0));
    assert_eq!(
        documentation.totals[1],
        format!("  {RETIRED_DOCS_ROOT}/: outside the scope")
    );
    let code = run(&fixture, &["apps"]);
    assert_eq!(outcome_counts(&code), (0, 1, 0));
    let retired = run(&fixture, &[RETIRED_DOCS_ROOT]);
    assert_eq!(outcome_counts(&retired), (0, 1, 0));
    let whole = run(&fixture, &[]);
    assert_eq!(outcome_counts(&whole), (1, 3, 0));
}

#[test]
fn a_failed_listing_or_an_empty_scope_did_not_run() {
    let failed = judge(
        Path::new("/nonexistent/documentation-gates"),
        Err(NotRun::ToolError {
            tool: "git ls-files -z".to_string(),
            status: 128,
            stderr: "fatal: not a git repository".to_string(),
        }),
        &[],
    );
    assert_eq!(
        failures(&failed)[0].lines().next(),
        Some(
            "FAIL: markdown-placement could not list the tracked files — git ls-files -z exited 128"
        )
    );
    assert_eq!(failed.print(), 2);

    let mut fixture = FixtureCheckout::new("placement-empty-scope");
    fixture
        .tracked("apps/README.md", "# Apps\n")
        .tracked(".ai/tickets/ROOT", "");
    let outside = run(&fixture, &[".ai"]);
    assert_eq!(outcome_counts(&outside), (0, 0, 1));
    assert!(failures(&outside)[0].starts_with("FAIL: markdown-placement judged nothing in .ai"));
    assert_eq!(outside.print(), 2);
}
