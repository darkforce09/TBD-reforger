use super::*;
use crate::UntrackedFiles;
use crate::fixture_checkout::{FixtureCheckout, failures, outcome_counts, request};
use repository_layout::{ARCHIVE_DIR, PENDING_MERGE_DIR, TICKET_DOCUMENTS_DIR};

fn run(fixture: &FixtureCheckout, scope: &[&str]) -> GateRun {
    judge(
        fixture.root(),
        Ok(fixture.tree()),
        &request(scope, UntrackedFiles::Invisible),
    )
}

fn lines(count: usize) -> String {
    "a line\n".repeat(count)
}

#[test]
fn code_trees_hold_only_readme_markdown() {
    let mut fixture = FixtureCheckout::new("placement-code");
    fixture
        .tracked("engines/README.md", "# Engines\n")
        .tracked("engines/tool/NOTES.md", "# Notes\n")
        .tracked("engines/tool/rules.MD", "# Rules\n")
        .tracked("engines/tool/tests/fixture.md", "")
        .tracked("engines/tool/generated/api.md", "")
        .tracked("engines/.cursor/guide.md", "")
        .tracked("engines/tool/main.rs", "");
    let run = run(&fixture, &[]);
    assert_eq!(
        failures(&run),
        [
            "FAIL: engines/tool/NOTES.md: Markdown in a code tree; a code tree holds only README.md, \
             and documents live under documentation/",
            "FAIL: engines/tool/rules.MD: Markdown in a code tree; a code tree holds only README.md, \
             and documents live under documentation/",
        ]
    );
    assert_eq!(
        outcome_counts(&run),
        (3, 2, 0),
        "README.md and the two empty retired folders held"
    );
    assert_eq!(
        run.totals[0],
        "  code trees: 3 Markdown file(s) judged, 2 other than README.md"
    );
    assert_eq!(run.print(), 1);
}

#[test]
fn generated_folder_exemption_spares_markdown_below_the_capitalised_folder_only() {
    let mut fixture = FixtureCheckout::new("placement-generated-spellings");
    fixture
        .tracked("engines/tool/Generated/weapon/table.md", "")
        .tracked("engines/tool/generated/api.md", "")
        .tracked("engines/tool/GENERATED/table.md", "")
        .tracked("engines/tool/generated_data/notes.md", "");
    let run = run(&fixture, &[]);
    assert_eq!(
        failures(&run),
        [
            "FAIL: engines/tool/GENERATED/table.md: Markdown in a code tree; a code tree holds only \
             README.md, and documents live under documentation/",
            "FAIL: engines/tool/generated_data/notes.md: Markdown in a code tree; a code tree holds \
             only README.md, and documents live under documentation/",
        ]
    );
    assert_eq!(
        run.totals[0],
        "  code trees: 2 Markdown file(s) judged, 2 other than README.md"
    );
}

/// Each retired top-level folder holds no tracked file: a tracked file under `docs/` or `apps/`
/// fails with where that folder's contents live now, and a tracked file there is never judged as
/// a code tree's Markdown.
#[test]
fn the_retired_folders_hold_no_tracked_file() {
    let mut fixture = FixtureCheckout::new("placement-retired");
    fixture
        .tracked("docs/guide.md", "# Guide\n")
        .tracked("docs/images/map.png", "")
        .tracked("engines/README.md", "# Engines\n");
    let docs_only = run(&fixture, &[]);
    assert_eq!(
        failures(&docs_only),
        [
            "FAIL: docs/ holds 2 tracked file(s); every document lives under documentation/\n      \
          docs/guide.md\n      docs/images/map.png"
        ]
    );
    assert_eq!(
        docs_only.totals[1],
        "  docs/: 2 tracked file(s); apps/: 0 tracked file(s)"
    );
    fixture
        .tracked("apps/README.md", "# Apps\n")
        .tracked("apps/tool/NOTES.md", "# Notes\n");
    let both = run(&fixture, &[]);
    assert_eq!(
        failures(&both)[1],
        "FAIL: apps/ holds 2 tracked file(s); the Enfusion mod lives under mod/ and every \
         application is a crate\n      apps/README.md\n      apps/tool/NOTES.md"
    );
    assert_eq!(failures(&both).len(), 2, "{:?}", failures(&both));
    assert_eq!(
        both.totals[0],
        "  code trees: 1 Markdown file(s) judged, 0 other than README.md"
    );
    assert_eq!(
        both.totals[1],
        "  docs/: 2 tracked file(s); apps/: 2 tracked file(s)"
    );
}

#[test]
fn live_documents_stay_at_or_under_the_limit() {
    let mut fixture = FixtureCheckout::new("placement-size");
    fixture
        .tracked("documentation/runbooks/short.md", &lines(500))
        .tracked("documentation/runbooks/long.md", &lines(501))
        .tracked("documentation/runbooks/table.csv", &lines(900));
    let run = run(&fixture, &[]);
    assert_eq!(
        failures(&run),
        [
            "FAIL: documentation/runbooks/long.md: 501 lines; a live document stays at or under \
          500, so split it by topic into a folder with a README.md index"
        ]
    );
    assert_eq!(
        run.totals[2],
        "  documentation/: 2 live document(s) judged, 1 over 500 lines, 0 unreadable"
    );
}

#[test]
fn frozen_and_pending_documents_are_outside_the_limit() {
    let mut fixture = FixtureCheckout::new("placement-exempt");
    let long = lines(900);
    for exempt in [
        format!("{TICKET_DOCUMENTS_DIR}/specs/t1_example.md"),
        format!("{ARCHIVE_DIR}/topic/old_plan.md"),
        format!("{PENDING_MERGE_DIR}/writer/source.md"),
    ] {
        fixture.tracked(&exempt, &long);
    }
    fixture.tracked("documentation/README.md", &lines(10));
    let run = run(&fixture, &[]);
    assert_eq!(failures(&run), Vec::<String>::new());
    assert_eq!(
        outcome_counts(&run),
        (3, 0, 0),
        "the root README and the two retired folders"
    );
}

#[test]
fn a_document_missing_from_the_disk_did_not_run() {
    let mut fixture = FixtureCheckout::new("placement-unreadable");
    fixture.listed_only("documentation/runbooks/gone.md");
    let run = run(&fixture, &[]);
    assert_eq!(outcome_counts(&run), (2, 0, 1));
    assert_eq!(
        run.totals[2],
        "  documentation/: 1 live document(s) judged, 0 over 500 lines, 1 unreadable"
    );
    assert_eq!(run.print(), 2);
}

#[test]
fn the_scope_narrows_every_rule() {
    let mut fixture = FixtureCheckout::new("placement-scope");
    fixture
        .tracked("engines/tool/NOTES.md", "# Notes\n")
        .tracked("docs/guide.md", "# Guide\n")
        .tracked("apps/tool/NOTES.md", "# Notes\n")
        .tracked("documentation/long.md", &lines(600))
        .tracked("documentation/area/short.md", &lines(5));
    let documentation = run(&fixture, &["documentation/area"]);
    assert_eq!(outcome_counts(&documentation), (1, 0, 0));
    assert_eq!(
        documentation.totals[1],
        "  docs/: outside the scope; apps/: outside the scope"
    );
    let code = run(&fixture, &["engines"]);
    assert_eq!(outcome_counts(&code), (0, 1, 0));
    let retired = run(&fixture, &["docs"]);
    assert_eq!(outcome_counts(&retired), (0, 1, 0));
    let retired_applications = run(&fixture, &["apps"]);
    assert_eq!(outcome_counts(&retired_applications), (0, 1, 0));
    let whole = run(&fixture, &[]);
    assert_eq!(outcome_counts(&whole), (1, 4, 0));
}

#[test]
fn untracked_markdown_is_placed_only_with_untracked_files_included() {
    let mut fixture = FixtureCheckout::new("placement-untracked");
    fixture
        .tracked(".gitignore", "build/\n")
        .tracked("engines/README.md", "# Engines\n")
        .untracked("engines/tool/NOTES.md", "# Notes\n")
        .untracked("engines/tool/build/report.md", "# Report\n")
        .untracked("documentation/runbooks/long.md", &lines(501));
    let run_listed_by_git = |untracked| {
        judge(
            fixture.root(),
            fixture.listed_by_git(untracked),
            &request(&[], untracked),
        )
    };
    let committed = run_listed_by_git(UntrackedFiles::Invisible);
    assert_eq!(failures(&committed), Vec::<String>::new());
    assert_eq!(committed.summary_label(), "markdown-placement");

    let with_untracked = run_listed_by_git(UntrackedFiles::Included);
    assert_eq!(
        failures(&with_untracked),
        [
            "FAIL: engines/tool/NOTES.md: Markdown in a code tree; a code tree holds only README.md, \
             and documents live under documentation/",
            "FAIL: documentation/runbooks/long.md: 501 lines; a live document stays at or \
             under 500, so split it by topic into a folder with a README.md index",
        ],
        "the ignored build/ report is never judged"
    );
    assert_eq!(
        with_untracked.summary_label(),
        "markdown-placement --with-untracked (untracked files included)"
    );
    assert_eq!(with_untracked.print(), 1);
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
        &GateRequest::default(),
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
        .tracked("engines/README.md", "# Engines\n")
        .tracked(".ai/tickets/ROOT", "");
    let outside = run(&fixture, &[".ai"]);
    assert_eq!(outcome_counts(&outside), (0, 0, 1));
    assert!(failures(&outside)[0].starts_with("FAIL: markdown-placement judged nothing in .ai"));
    assert_eq!(outside.print(), 2);
}

/// Every top-level folder but the documentation root and the retired folders is a code tree, so
/// Markdown in a folder no list names is placed like Markdown in any other code tree; the
/// repository root and hidden folders are not code trees.
#[test]
fn markdown_in_a_top_level_folder_no_list_names_is_placed() {
    let mut fixture = FixtureCheckout::new("placement-derived-code-trees");
    fixture
        .tracked("README.md", "# Project\n")
        .tracked("CLAUDE.md", "# Instructions\n")
        .tracked("deploy/README.md", "# Deploy\n")
        .tracked("deploy/NOTES.md", "# Notes\n")
        .tracked("engines/map_tool/guide.md", "# Guide\n")
        .tracked(".github/pull_request_template.md", "# Template\n");
    let run = run(&fixture, &[]);
    assert_eq!(
        failures(&run),
        [
            "FAIL: deploy/NOTES.md: Markdown in a code tree; a code tree holds only README.md, \
             and documents live under documentation/",
            "FAIL: engines/map_tool/guide.md: Markdown in a code tree; a code tree holds only \
             README.md, and documents live under documentation/",
        ]
    );
    assert_eq!(
        run.totals[0],
        "  code trees: 3 Markdown file(s) judged, 2 other than README.md"
    );
    assert_eq!(run.print(), 1);
}
