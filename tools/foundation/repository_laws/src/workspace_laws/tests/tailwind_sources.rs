//! Tests for [`super`] — one exact `@source` line per leptos crate, and no stale line.

use super::*;
use crate::temporary_checkout::this_repository;
use crate::workspace_laws::fixture_workspace::{FixtureWorkspace, application_manifest, normal};

const STYLESHEET: &str = "crates/frontend/shell/frontend_application/style/app.css";
const APP_LINE: &str = "@source \"../src/**/*.rs\";\n";
const PAGES_LINE: &str =
    "@source '../../../../../crates/frontend/pages/account_pages/src/**/*.rs';\n";

fn workspace(name: &str, stylesheet: &str) -> FixtureWorkspace {
    let mut workspace = FixtureWorkspace::new(name);
    workspace.member(
        "crates/frontend/shell/frontend_application",
        &application_manifest("frontend_application", "leptos = \"0.8\"\n"),
    );
    workspace.layout_crate(
        "crates/frontend/pages/account_pages",
        0,
        "any",
        &[normal("leptos")],
    );
    workspace.layout_crate("crates/mission/mission_model", 0, "any", &[]);
    workspace.write(STYLESHEET, stylesheet);
    workspace
}

fn findings(workspace: &FixtureWorkspace) -> Vec<String> {
    tailwind_sources_outcome(workspace.root(), STYLESHEET)
        .unwrap()
        .findings
}

/// The finding of the account pages crate that no line names.
fn unnamed_account_pages() -> String {
    format!(
        "crates/frontend/pages/account_pages depends on leptos but no @source line of \
         {STYLESHEET} names crates/frontend/pages/account_pages/src/**/*.rs (one line per crate, \
         its own src folder)"
    )
}

/// The finding of the stale line `glob`.
fn stale(glob: &str) -> String {
    format!("@source \"{glob}\" in {STYLESHEET} names no leptos member's src/**/*.rs: a stale line")
}

#[test]
fn tailwind_sources_one_exact_line_per_leptos_crate_passes() {
    let workspace = workspace(
        "tailwind-green",
        &format!("@import 'tailwindcss';\n{APP_LINE}{PAGES_LINE}@source not \"../dist\";\n"),
    );
    let report = check_tailwind_sources(workspace.root(), STYLESHEET);
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert!(report.lines[0].contains("2 leptos member(s), 2 @source glob(s)"));
}

#[test]
fn tailwind_sources_a_leptos_crate_without_its_line_is_a_finding() {
    let workspace = workspace(
        "tailwind-missing-line",
        &format!(
            "{APP_LINE}@source not \"../../../../../crates/frontend/pages/account_pages/src/**/*.rs\";\n"
        ),
    );
    assert_eq!(findings(&workspace), vec![unnamed_account_pages()]);
    assert_eq!(
        check_tailwind_sources(workspace.root(), STYLESHEET).exit_code,
        1
    );
    workspace.write(STYLESHEET, &format!("{APP_LINE}{PAGES_LINE}"));
    assert_eq!(findings(&workspace), Vec::<String>::new());
}

#[test]
fn tailwind_sources_an_ancestor_or_wildcard_folder_no_longer_covers_and_is_stale() {
    for (index, glob) in [
        "../../../../../crates/frontend/**/*.rs",
        "../../../../../crates/frontend/*/*/src/**/*.rs",
        "../../../../../crates/frontend/pages/account_pages/src/*.rs",
    ]
    .into_iter()
    .enumerate()
    {
        let workspace = workspace(
            &format!("tailwind-ancestor-{index}"),
            &format!("{APP_LINE}@source \"{glob}\";\n"),
        );
        assert_eq!(
            findings(&workspace),
            vec![unnamed_account_pages(), stale(glob)],
            "{glob}"
        );
    }
}

#[test]
fn tailwind_sources_a_stale_or_duplicate_line_is_a_finding() {
    let workspace = workspace(
        "tailwind-stale",
        &format!(
            "{APP_LINE}{PAGES_LINE}@source \"../../../../../crates/mission/mission_model/src/**/*.rs\";\n\
             @source \"../../../../../crates/frontend/pages/retired_pages/src/**/*.rs\";\n\
             @source \"../../../../../../../outside/src/**/*.rs\";\n@source \"./../src/**/*.rs\";\n"
        ),
    );
    assert_eq!(
        findings(&workspace),
        vec![
            format!(
                "2 @source lines of {STYLESHEET} name crates/frontend/shell/frontend_application/src/**/*.rs; keep one"
            ),
            stale("../../../../../crates/mission/mission_model/src/**/*.rs"),
            stale("../../../../../crates/frontend/pages/retired_pages/src/**/*.rs"),
            stale("../../../../../../../outside/src/**/*.rs"),
        ]
    );
}

#[test]
fn tailwind_sources_a_dev_only_leptos_dependency_needs_no_line() {
    let mut workspace = workspace("tailwind-dev-only", &format!("{APP_LINE}{PAGES_LINE}"));
    workspace.member(
        "tools/checks/view_probe",
        "[package]\nname = \"view_probe\"\n\n[dev-dependencies]\nleptos = \"0.8\"\n",
    );
    assert_eq!(findings(&workspace), Vec::<String>::new());
}

#[test]
fn tailwind_sources_a_missing_stylesheet_did_not_run() {
    let workspace = workspace("tailwind-missing", "");
    std::fs::remove_file(workspace.root().join(STYLESHEET)).unwrap();
    assert_eq!(
        check_tailwind_sources(workspace.root(), STYLESHEET).exit_code,
        2
    );
}

#[test]
fn tailwind_sources_reads_quoted_globs_and_skips_not_lines() {
    let globs = source_globs(
        "@source \"a/**/*.rs\";\n  @source 'b';\n@source not \"c\";\n@sources \"d\";\n",
    );
    assert_eq!(globs, vec!["a/**/*.rs".to_string(), "b".to_string()]);
}

#[test]
fn tailwind_sources_this_checkout_passes() {
    let report = check_tailwind_sources(
        &this_repository(),
        "crates/frontend/shell/frontend_application/style/aegis.css",
    );
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
}
