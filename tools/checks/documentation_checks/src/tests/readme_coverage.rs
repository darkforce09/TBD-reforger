use super::*;
use crate::UntrackedFiles;
use crate::fixture_checkout::{
    FixtureCheckout, failures, outcome_counts, readme_with_contents, request,
};
use crate::path_regions::file_name;
use repository_layout::PENDING_MERGE_DIR;
use repository_layout::documentation::DOCUMENTATION_ROOT;

fn run(fixture: &FixtureCheckout, scope: &[&str]) -> GateRun {
    judge(
        fixture.root(),
        Ok(fixture.tree()),
        &request(scope, UntrackedFiles::Invisible),
    )
}

/// The gate over the whole checkout as the real listing sees it.
fn run_listed_by_git(fixture: &FixtureCheckout, untracked: UntrackedFiles) -> GateRun {
    judge(
        fixture.root(),
        fixture.listed_by_git(untracked),
        &request(&[], untracked),
    )
}

fn no_failures() -> Vec<String> {
    Vec::new()
}

#[test]
fn folders_with_matching_readmes_hold_both_rules() {
    let mut fixture = FixtureCheckout::new("coverage-clean");
    fixture
        .tracked(
            "engines/README.md",
            &readme_with_contents("engines/", &["└── tool/  the tool"]),
        )
        .tracked(
            "engines/tool/README.md",
            &readme_with_contents(
                "engines/tool/",
                &["├── main.rs  entry point", "└── tests/  unit tests"],
            ),
        )
        .tracked("engines/tool/main.rs", "fn main() {}\n")
        .tracked("engines/tool/tests/cases.rs", "\n");
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
            "engines/README.md",
            &readme_with_contents("engines/", &["└── tool/  the tool"]),
        )
        .tracked("engines/tool/main.rs", "fn main() {}\n");
    let run = run(&fixture, &[]);
    assert_eq!(
        failures(&run),
        ["FAIL: engines/tool/: no tracked README.md"]
    );
    assert_eq!(run.print(), 1);
}

#[test]
fn test_generated_hidden_and_pending_merge_folders_need_no_readme() {
    let mut fixture = FixtureCheckout::new("coverage-skipped");
    fixture
        .tracked(
            "engines/README.md",
            &readme_with_contents(
                "engines/",
                &[
                    "├── .config/     tool configuration",
                    "├── generated/   generated models",
                    "└── tests/       unit tests",
                ],
            ),
        )
        .tracked("engines/.config/settings.toml", "")
        .tracked("engines/generated/models/model.rs", "")
        .tracked("engines/tests/deep/case.rs", "")
        .tracked(
            &format!("{PENDING_MERGE_DIR}/writer/source.md"),
            "# Source\n",
        )
        .tracked(
            "documentation/README.md",
            &readme_with_contents(
                "documentation/",
                &["└── pending_merge/  sources waiting to merge"],
            ),
        );
    let run = run(&fixture, &[]);
    assert_eq!(failures(&run), no_failures());
    assert_eq!(
        outcome_counts(&run),
        (4, 0, 0),
        "engines and documentation only"
    );
}

#[test]
fn generated_folder_exemption_spares_the_capitalised_folder_and_no_other_spelling() {
    let mut fixture = FixtureCheckout::new("coverage-generated-spellings");
    fixture
        .tracked(
            "engines/README.md",
            &readme_with_contents(
                "engines/",
                &[
                    "├── GENERATED/       tables in capitals",
                    "├── Generated/       generated script tables",
                    "├── generated/       generated models",
                    "└── generated_data/  inputs a generator reads",
                ],
            ),
        )
        .tracked("engines/Generated/weapon/table.c", "")
        .tracked("engines/generated/models/model.rs", "")
        .tracked("engines/GENERATED/table.c", "")
        .tracked("engines/generated_data/input.json", "");
    let run = run(&fixture, &[]);
    assert_eq!(
        failures(&run),
        [
            "FAIL: engines/GENERATED/: no tracked README.md",
            "FAIL: engines/generated_data/: no tracked README.md",
        ]
    );
    assert_eq!(
        run.totals[0],
        "  coverage: 3 folder(s) judged, 2 without a tracked README.md"
    );
}

#[test]
fn a_readme_inside_an_exempt_folder_is_not_held_to_its_contents() {
    const NO_CONTENTS: &str = "# Exempt\n\nNo Contents here.\n";
    let pending_merge = file_name(PENDING_MERGE_DIR);
    let mut fixture = FixtureCheckout::new("coverage-exempt-readme");
    fixture
        .tracked(
            "engines/README.md",
            &readme_with_contents(
                "engines/",
                &[
                    "├── .cfg/       tool configuration",
                    "├── generated/  generated models",
                    "├── tests/      unit tests",
                    "└── tool/       the tool",
                ],
            ),
        )
        .tracked("engines/.cfg/README.md", NO_CONTENTS)
        .tracked("engines/generated/README.md", NO_CONTENTS)
        .tracked("engines/tests/README.md", NO_CONTENTS)
        .tracked("engines/tests/case.rs", "")
        .tracked("engines/tool/README.md", NO_CONTENTS)
        .tracked("engines/tool/main.rs", "")
        .tracked(
            &format!("{DOCUMENTATION_ROOT}/README.md"),
            &readme_with_contents(
                &format!("{DOCUMENTATION_ROOT}/"),
                &[format!("└── {pending_merge}/  sources waiting to merge").as_str()],
            ),
        )
        .tracked(&format!("{PENDING_MERGE_DIR}/README.md"), NO_CONTENTS);
    let whole = run(&fixture, &[]);
    assert_eq!(
        failures(&whole),
        [
            "FAIL: engines/tool/README.md: Contents does not match the folder (1 violation(s))\n      \
          engines/tool/README.md:1: no `## Contents` heading"
        ],
        "only the README of a folder in the span is judged"
    );
    assert_eq!(outcome_counts(&whole), (5, 1, 0));
    let inside_exempt = run(&fixture, &["engines/tests"]);
    assert_eq!(outcome_counts(&inside_exempt), (0, 0, 1));
    assert!(
        failures(&inside_exempt)[0]
            .starts_with("FAIL: readme-coverage judged nothing in engines/tests")
    );
}

#[test]
fn every_contents_violation_prints_as_path_line_message() {
    let mut fixture = FixtureCheckout::new("coverage-violations");
    fixture
        .tracked(
            "engines/README.md",
            &readme_with_contents("engines", &["├── main.rs  entry", "└── gone.rs  deleted"]),
        )
        .tracked("engines/main.rs", "")
        .tracked("engines/extra.rs", "");
    let run = run(&fixture, &[]);
    assert_eq!(
        failures(&run),
        [
            "FAIL: engines/README.md: Contents does not match the folder (3 violation(s))\n      \
          engines/README.md:8: the root line is `engines`, not the folder path `engines/`\n      \
          engines/README.md:8: tracked child `extra.rs` matches no entry\n      \
          engines/README.md:10: entry `gone.rs` matches no tracked child"
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
            "engines/README.md",
            &readme_with_contents("engines/", &["└── main.rs  entry point"]),
        )
        .tracked("engines/main.rs", "")
        .untracked("engines/scratch.rs", "")
        .untracked("engines/notes/README.md", "# Notes\n");
    let run = run(&fixture, &[]);
    assert_eq!(failures(&run), no_failures());
    assert_eq!(outcome_counts(&run), (2, 0, 0));
}

#[test]
fn an_untracked_readme_makes_its_folder_pass_only_with_untracked_files_included() {
    let mut fixture = FixtureCheckout::new("coverage-untracked-readme");
    fixture
        .tracked(
            "engines/README.md",
            &readme_with_contents("engines/", &["└── tool/  the tool"]),
        )
        .tracked("engines/tool/main.rs", "fn main() {}\n")
        .untracked(
            "engines/tool/README.md",
            &readme_with_contents("engines/tool/", &["└── main.rs  entry point"]),
        );
    let committed = run_listed_by_git(&fixture, UntrackedFiles::Invisible);
    assert_eq!(
        failures(&committed),
        ["FAIL: engines/tool/: no tracked README.md"]
    );
    assert_eq!(committed.summary_label(), "readme-coverage");
    assert_eq!(committed.print(), 1);

    let with_untracked = run_listed_by_git(&fixture, UntrackedFiles::Included);
    assert_eq!(failures(&with_untracked), no_failures());
    assert_eq!(outcome_counts(&with_untracked), (4, 0, 0));
    assert_eq!(
        with_untracked.header[1],
        "    scope: the whole repository; git listed 2 tracked file(s) and 1 untracked file(s) it \
         does not ignore"
    );
    assert_eq!(
        with_untracked.summary_label(),
        "readme-coverage --with-untracked (untracked files included)"
    );
    assert_eq!(with_untracked.print(), 0);
}

#[test]
fn an_untracked_folder_is_a_child_only_with_untracked_files_included_and_an_ignored_one_never_is() {
    let mut fixture = FixtureCheckout::new("coverage-untracked-folder");
    fixture
        .tracked(".gitignore", "build/\n")
        .tracked(
            "engines/README.md",
            &readme_with_contents(
                "engines/",
                &["├── fresh/   the new tool", "└── main.rs  entry"],
            ),
        )
        .tracked("engines/main.rs", "")
        .untracked(
            "engines/fresh/README.md",
            &readme_with_contents("engines/fresh/", &["└── lib.rs  the library"]),
        )
        .untracked("engines/fresh/lib.rs", "")
        .untracked("engines/build/output.txt", "");
    let committed = run_listed_by_git(&fixture, UntrackedFiles::Invisible);
    assert_eq!(
        failures(&committed),
        [
            "FAIL: engines/README.md: Contents does not match the folder (1 violation(s))\n      \
          engines/README.md:9: entry `fresh/` matches no tracked child"
        ]
    );
    assert_eq!(outcome_counts(&committed), (1, 1, 0));

    let with_untracked = run_listed_by_git(&fixture, UntrackedFiles::Included);
    assert_eq!(
        failures(&with_untracked),
        no_failures(),
        "the ignored build/ folder is neither a child nor a folder without a README"
    );
    assert_eq!(outcome_counts(&with_untracked), (4, 0, 0));
}

#[test]
fn the_scope_narrows_the_judged_folders() {
    let mut fixture = FixtureCheckout::new("coverage-scope");
    fixture
        .tracked("engines/README.md", "# Engines\n\nNo Contents.\n")
        .tracked(
            "engines/tool/README.md",
            &readme_with_contents("engines/tool/", &["└── main.rs  entry point"]),
        )
        .tracked("engines/tool/main.rs", "");
    let scoped = run(&fixture, &["engines/tool"]);
    assert_eq!(failures(&scoped), no_failures());
    assert_eq!(outcome_counts(&scoped), (2, 0, 0));
    assert_eq!(
        scoped.header[1],
        "    scope: engines/tool; git listed 3 tracked file(s)"
    );
    let whole = run(&fixture, &[]);
    assert_eq!(outcome_counts(&whole), (3, 1, 0));
}

#[test]
fn a_tracked_readme_missing_from_the_disk_did_not_run() {
    let mut fixture = FixtureCheckout::new("coverage-unreadable");
    fixture
        .listed_only("engines/README.md")
        .tracked("engines/main.rs", "");
    let run = run(&fixture, &[]);
    assert_eq!(
        outcome_counts(&run),
        (1, 0, 1),
        "coverage held, contents did not run"
    );
    assert!(
        failures(&run)[0]
            .starts_with("FAIL: engines/README.md could not be read — target file missing: ")
    );
    assert_eq!(run.print(), 2);
}

#[test]
fn a_failed_or_empty_listing_did_not_run() {
    let root = Path::new("/nonexistent/documentation-gates");
    let absent = judge(
        root,
        Err(NotRun::ToolAbsent("git".to_string())),
        &GateRequest::default(),
    );
    assert_eq!(
        failures(&absent)[0].lines().next(),
        Some("FAIL: readme-coverage could not list the tracked files — git not found")
    );
    assert_eq!(absent.print(), 2);
    let empty = judge(
        root,
        Ok(TrackedTree::from_listing("")),
        &GateRequest::default(),
    );
    assert_eq!(outcome_counts(&empty), (0, 0, 1));
    assert_eq!(empty.print(), 2);
}

#[test]
fn a_run_that_stopped_still_marks_untracked_files_on_its_summary() {
    let untracked = request(&[], UntrackedFiles::Included);
    let absent = judge(
        Path::new("/nonexistent/documentation-gates"),
        Err(NotRun::ToolAbsent("git".to_string())),
        &untracked,
    );
    assert_eq!(
        failures(&absent)[0].lines().next(),
        Some(
            "FAIL: readme-coverage could not list the tracked and untracked files — git not found"
        )
    );
    assert_eq!(
        absent.summary_label(),
        "readme-coverage --with-untracked (untracked files included)"
    );
    assert_eq!(absent.print(), 2);
}

#[test]
fn a_refused_or_empty_scope_did_not_run() {
    let mut fixture = FixtureCheckout::new("coverage-bad-scope");
    fixture
        .tracked(
            "engines/README.md",
            &readme_with_contents("engines/", &["└── main.rs  entry point"]),
        )
        .tracked("engines/main.rs", "")
        .tracked(".ai/tickets/ROOT", "");
    let refused = run(&fixture, &["engines/missing"]);
    assert_eq!(outcome_counts(&refused), (0, 0, 1));
    assert!(
        failures(&refused)[0]
            .starts_with("FAIL: readme-coverage scope `engines/missing` names no tracked folder")
    );
    let outside = run(&fixture, &[".ai"]);
    assert_eq!(outcome_counts(&outside), (0, 0, 1));
    assert!(failures(&outside)[0].starts_with("FAIL: readme-coverage judged nothing in .ai"));
    assert_eq!(outside.print(), 2);
}

/// The span comes from the listing, not from a list of names: a top-level folder nothing names
/// is judged like `engines/`, while the repository root, hidden folders and the retired top-level
/// folders (`docs`, `apps`), tracked again here, are not: markdown-placement fails them instead.
#[test]
fn a_top_level_folder_no_list_names_is_judged_by_default() {
    let mut fixture = FixtureCheckout::new("coverage-derived-span");
    fixture
        .tracked("README.md", "# Project\n")
        .tracked("Cargo.toml", "")
        .tracked(
            "engines/README.md",
            &readme_with_contents("engines/", &["└── main.rs  entry point"]),
        )
        .tracked("engines/main.rs", "")
        .tracked(
            "a_folder_born_later/README.md",
            &readme_with_contents("a_folder_born_later/", &["└── settings.toml  settings"]),
        )
        .tracked("a_folder_born_later/settings.toml", "")
        .tracked("deploy/compose.yml", "")
        .tracked(".github/workflows/ci.yml", "")
        .tracked("docs/images/map.png", "")
        .tracked("apps/tool/notes.txt", "");
    let run = run(&fixture, &[]);
    assert_eq!(failures(&run), ["FAIL: deploy/: no tracked README.md"]);
    assert_eq!(
        outcome_counts(&run),
        (4, 1, 0),
        "engines and a_folder_born_later hold both rules, deploy fails coverage"
    );
    assert_eq!(
        run.header[0],
        "==> readme-coverage: every folder below the repository root carries a README.md whose \
         Contents block matches it (top-level folders: a_folder_born_later, deploy, engines)"
    );
    assert_eq!(run.print(), 1);
}
