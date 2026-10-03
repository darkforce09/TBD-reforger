//! Tests for [`super`] — `@source` coverage of the leptos crates.

use super::*;
use crate::temporary_checkout::this_repository;
use crate::workspace_laws::fixture_workspace::{FixtureWorkspace, application_manifest, normal};

const STYLESHEET: &str = "apps/frontend/style/app.css";

fn workspace(name: &str, stylesheet: &str) -> FixtureWorkspace {
    let mut workspace = FixtureWorkspace::new(name);
    workspace.member(
        "apps/frontend",
        &application_manifest("frontend", "leptos = \"0.8\"\n"),
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

#[test]
fn tailwind_sources_every_leptos_crate_covered_passes() {
    let workspace = workspace(
        "tailwind-green",
        "@import 'tailwindcss';\n@source \"../src/**/*.rs\";\n@source '../../../crates/frontend/*/*/src/**/*.rs';\n",
    );
    let report = check_tailwind_sources(workspace.root(), STYLESHEET);
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert!(report.lines[0].contains("2 leptos member(s), 2 @source glob(s)"));
}

#[test]
fn tailwind_sources_a_leptos_crate_without_a_source_line_is_a_finding() {
    let workspace = workspace(
        "tailwind-red",
        "@source \"../src/**/*.rs\";\n@source not \"../../../crates/frontend/**/*.rs\";\n",
    );
    let found = tailwind_sources_outcome(workspace.root(), STYLESHEET)
        .unwrap()
        .findings;
    assert_eq!(
        found,
        vec![format!(
            "crates/frontend/pages/account_pages depends on leptos but no @source line of {STYLESHEET} \
         covers crates/frontend/pages/account_pages/src/**/*.rs"
        )]
    );
    workspace.write(
        STYLESHEET,
        "@source \"../src/**/*.rs\";\n@source \"../../../crates/frontend/**/*.rs\";\n",
    );
    assert_eq!(
        check_tailwind_sources(workspace.root(), STYLESHEET).exit_code,
        0,
        "an ancestor folder covers"
    );
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
    let report = check_tailwind_sources(&this_repository(), "apps/frontend/style/aegis.css");
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
}
