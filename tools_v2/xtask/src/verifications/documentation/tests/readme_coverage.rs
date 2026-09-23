use super::super::fixture_checkout::{
    FixtureCheckout, failures, outcome_counts, readme_with_contents,
};
use super::*;
use crate::core::repository_layout::documentation::PENDING_MERGE_DIR;

fn run(fixture: &FixtureCheckout, scope: &[&str]) -> GateRun {
    let scope: Vec<String> = scope.iter().map(ToString::to_string).collect();
    judge(fixture.root(), Ok(fixture.tree()), &scope)
}

fn no_failures() -> Vec<String> {
    Vec::new()
}

#[test]
fn folders_with_matching_readmes_hold_both_rules() {
    let mut fixture = FixtureCheckout::new("coverage-clean");
    fixture
        .tracked(
            "apps/README.md",
            &readme_with_contents("apps/", &["└── tool/  the tool"]),
        )
        .tracked(
            "apps/tool/README.md",
            &readme_with_contents(
                "apps/tool/",
                &["├── main.rs  entry point", "└── tests/  unit tests"],
            ),
        )
        .tracked("apps/tool/main.rs", "fn main() {}\n")
        .tracked("apps/tool/tests/cases.rs", "\n");
    let run = run(&fixture, &[]);
    assert_eq!(failures(&run), no_failures());
    assert_eq!(outcome_counts(&run), (4, 0, 0), "two folders, two READMEs");
    assert_eq!(
        run.totals,
        [
            "  coverage: 2 folder(s) judged, 0 without a tracked README.md",
            "  contents: 2 README.md file(s) judged, 0 not matching their folder (0 violation(s)), \
             0 unreadable",
        ]
    );
    assert_eq!(run.print(), 0);
}

#[test]
fn a_folder_without_a_readme_fails_coverage() {
    let mut fixture = FixtureCheckout::new("coverage-missing");
    fixture
        .tracked(
            "apps/README.md",
            &readme_with_contents("apps/", &["└── tool/  the tool"]),
        )
        .tracked("apps/tool/main.rs", "fn main() {}\n");
    let run = run(&fixture, &[]);
    assert_eq!(failures(&run), ["FAIL: apps/tool/: no tracked README.md"]);
    assert_eq!(run.print(), 1);
}

#[test]
fn test_generated_hidden_and_pending_merge_folders_need_no_readme() {
    let mut fixture = FixtureCheckout::new("coverage-skipped");
    fixture
        .tracked(
            "apps/README.md",
            &readme_with_contents(
                "apps/",
                &[
                    "├── .config/     tool configuration",
                    "├── generated/   generated models",
                    "└── tests/       unit tests",
                ],
            ),
        )
        .tracked("apps/.config/settings.toml", "")
        .tracked("apps/generated/models/model.rs", "")
        .tracked("apps/tests/deep/case.rs", "")
        .tracked(
            &format!("{PENDING_MERGE_DIR}/writer/source.md"),
            "# Source\n",
        )
        .tracked(
            "documentation_v2/README.md",
            &readme_with_contents(
                "documentation_v2/",
                &["└── pending_merge/  sources waiting to merge"],
            ),
        );
    let run = run(&fixture, &[]);
    assert_eq!(failures(&run), no_failures());
    assert_eq!(
        outcome_counts(&run),
        (4, 0, 0),
        "apps and documentation_v2 only"
    );
}

#[test]
fn a_readme_inside_a_skipped_folder_is_still_held_to_its_contents() {
    let mut fixture = FixtureCheckout::new("coverage-skipped-readme");
    fixture
        .tracked(
            "apps/README.md",
            &readme_with_contents("apps/", &["└── tests/  unit tests"]),
        )
        .tracked("apps/tests/README.md", "# Tests\n\nNo Contents here.\n")
        .tracked("apps/tests/case.rs", "");
    let run = run(&fixture, &[]);
    assert_eq!(
        failures(&run),
        [
            "FAIL: apps/tests/README.md: Contents does not match the folder (1 violation(s))\n      \
          apps/tests/README.md:1: no `## Contents` heading"
        ]
    );
}

#[test]
fn every_contents_violation_prints_as_path_line_message() {
    let mut fixture = FixtureCheckout::new("coverage-violations");
    fixture
        .tracked(
            "apps/README.md",
            &readme_with_contents("apps", &["├── main.rs  entry", "└── gone.rs  deleted"]),
        )
        .tracked("apps/main.rs", "")
        .tracked("apps/extra.rs", "");
    let run = run(&fixture, &[]);
    assert_eq!(
        failures(&run),
        [
            "FAIL: apps/README.md: Contents does not match the folder (3 violation(s))\n      \
          apps/README.md:8: the root line is `apps`, not the folder path `apps/`\n      \
          apps/README.md:8: tracked child `extra.rs` matches no entry\n      \
          apps/README.md:10: entry `gone.rs` matches no tracked child"
        ]
    );
    assert_eq!(
        run.totals[1],
        "  contents: 1 README.md file(s) judged, 1 not matching their folder (3 violation(s)), \
         0 unreadable"
    );
}

#[test]
fn untracked_files_are_neither_children_nor_readmes() {
    let mut fixture = FixtureCheckout::new("coverage-untracked");
    fixture
        .tracked(
            "apps/README.md",
            &readme_with_contents("apps/", &["└── main.rs  entry point"]),
        )
        .tracked("apps/main.rs", "")
        .untracked("apps/scratch.rs", "")
        .untracked("apps/notes/README.md", "# Notes\n");
    let run = run(&fixture, &[]);
    assert_eq!(failures(&run), no_failures());
    assert_eq!(outcome_counts(&run), (2, 0, 0));
}

#[test]
fn the_scope_narrows_the_judged_folders() {
    let mut fixture = FixtureCheckout::new("coverage-scope");
    fixture
        .tracked("apps/README.md", "# Apps\n\nNo Contents.\n")
        .tracked(
            "apps/tool/README.md",
            &readme_with_contents("apps/tool/", &["└── main.rs  entry point"]),
        )
        .tracked("apps/tool/main.rs", "");
    let scoped = run(&fixture, &["apps/tool"]);
    assert_eq!(failures(&scoped), no_failures());
    assert_eq!(outcome_counts(&scoped), (2, 0, 0));
    assert_eq!(
        scoped.header[1],
        "    scope: apps/tool; git listed 3 tracked file(s)"
    );
    let whole = run(&fixture, &[]);
    assert_eq!(outcome_counts(&whole), (3, 1, 0));
}

#[test]
fn a_tracked_readme_missing_from_the_disk_did_not_run() {
    let mut fixture = FixtureCheckout::new("coverage-unreadable");
    fixture
        .listed_only("apps/README.md")
        .tracked("apps/main.rs", "");
    let run = run(&fixture, &[]);
    assert_eq!(
        outcome_counts(&run),
        (1, 0, 1),
        "coverage held, contents did not run"
    );
    assert!(
        failures(&run)[0]
            .starts_with("FAIL: apps/README.md could not be read — target file missing: ")
    );
    assert_eq!(run.print(), 2);
}

#[test]
fn a_failed_or_empty_listing_did_not_run() {
    let root = Path::new("/nonexistent/documentation-gates");
    let absent = judge(root, Err(NotRun::ToolAbsent("git".to_string())), &[]);
    assert_eq!(
        failures(&absent)[0].lines().next(),
        Some("FAIL: readme-coverage could not list the tracked files — git not found")
    );
    assert_eq!(absent.print(), 2);
    let empty = judge(root, Ok(TrackedTree::from_listing("")), &[]);
    assert_eq!(outcome_counts(&empty), (0, 0, 1));
    assert_eq!(empty.print(), 2);
}

#[test]
fn a_refused_or_empty_scope_did_not_run() {
    let mut fixture = FixtureCheckout::new("coverage-bad-scope");
    fixture
        .tracked(
            "apps/README.md",
            &readme_with_contents("apps/", &["└── main.rs  entry point"]),
        )
        .tracked("apps/main.rs", "")
        .tracked(".ai/tickets/ROOT", "");
    let refused = run(&fixture, &["apps/missing"]);
    assert_eq!(outcome_counts(&refused), (0, 0, 1));
    assert!(
        failures(&refused)[0]
            .starts_with("FAIL: readme-coverage scope `apps/missing` names no tracked folder")
    );
    let outside = run(&fixture, &[".ai"]);
    assert_eq!(outcome_counts(&outside), (0, 0, 1));
    assert!(failures(&outside)[0].starts_with("FAIL: readme-coverage judged nothing in .ai"));
    assert_eq!(outside.print(), 2);
}
