//! Tests for [`super`] — test files a member's module tree reaches, and the ones it does not.

use super::*;
use crate::workspace_laws::fixture_workspace::{FixtureWorkspace, application_manifest};

const CRATE: &str = "crates/mission/mission_model";

/// A library crate whose sibling unit tests and integration support files are all reached.
fn green_workspace(name: &str) -> FixtureWorkspace {
    let mut workspace = FixtureWorkspace::new(name);
    workspace.layout_crate(CRATE, 0, "any", &[]);
    workspace.write(
        &format!("{CRATE}/src/lib.rs"),
        "//! A fixture crate.\n\npub mod model;\npub mod prelude;\n",
    );
    workspace.write(
        &format!("{CRATE}/src/model.rs"),
        "//! Model.\n\n#[cfg(test)]\n#[path = \"tests/model.rs\"]\nmod tests;\n",
    );
    workspace.write(
        &format!("{CRATE}/src/tests/model.rs"),
        "use super::*;\nmod cases;\n",
    );
    workspace.write(
        &format!("{CRATE}/src/tests/cases.rs"),
        "#[test]\nfn t() {}\n",
    );
    workspace.write(&format!("{CRATE}/tests/round_trip.rs"), "mod support;\n");
    workspace.write(&format!("{CRATE}/tests/support/mod.rs"), "pub mod data;\n");
    workspace.write(&format!("{CRATE}/tests/support/data.rs"), "pub fn d() {}\n");
    workspace.write(&format!("{CRATE}/tests/suite/main.rs"), "mod cases;\n");
    workspace.write(
        &format!("{CRATE}/tests/suite/cases.rs"),
        "#[test]\nfn t() {}\n",
    );
    workspace
}

fn findings(workspace: &FixtureWorkspace) -> Vec<String> {
    test_file_reachability_outcome(workspace.root())
        .expect("the law runs")
        .findings
}

#[test]
fn test_file_reachability_a_workspace_whose_test_files_are_all_declared_passes() {
    let workspace = green_workspace("reachability-green");
    let report = check_test_file_reachability(workspace.root());
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert_eq!(
        report.lines.first().map(String::as_str),
        Some("==> test-file-reachability — 1 workspace member(s), 5 file(s) in test folders")
    );
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("TEST-FILE-REACHABILITY: PASS")
    );
}

#[test]
fn test_file_reachability_an_undeclared_sibling_test_file_is_a_finding() {
    let workspace = green_workspace("reachability-sibling");
    workspace.write(
        &format!("{CRATE}/src/tests/orphan.rs"),
        "#[test]\nfn never_runs() {}\n",
    );
    let found = findings(&workspace);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].starts_with(&format!(
            "{CRATE}/src/tests/orphan.rs: no module declaration of `mission_model` reaches"
        )),
        "{found:#?}"
    );
}

#[test]
fn test_file_reachability_an_undeclared_integration_support_file_is_a_finding() {
    let workspace = green_workspace("reachability-support");
    workspace.write(
        &format!("{CRATE}/tests/support/stale.rs"),
        "pub fn s() {}\n",
    );
    workspace.write(&format!("{CRATE}/tests/suite/lost.rs"), "pub fn l() {}\n");
    let found = findings(&workspace);
    assert_eq!(found.len(), 2, "{found:#?}");
    for file in ["tests/support/stale.rs", "tests/suite/lost.rs"] {
        assert!(
            found
                .iter()
                .any(|f| f.starts_with(&format!("{CRATE}/{file}:"))),
            "{file}: {found:#?}"
        );
    }
}

/// A file declared only from a file that is itself unreachable is unreachable too, and a
/// declaration quoted in a string or a comment reaches nothing.
#[test]
fn test_file_reachability_a_declaration_from_an_unreached_file_or_a_string_counts_for_nothing() {
    let workspace = green_workspace("reachability-chain");
    workspace.write(
        &format!("{CRATE}/src/unused.rs"),
        "#[path = \"tests/chained.rs\"]\nmod tests;\n",
    );
    workspace.write(&format!("{CRATE}/src/tests/chained.rs"), "\n");
    workspace.write(
        &format!("{CRATE}/src/model.rs"),
        "//! Model. mod quoted;\n\n#[cfg(test)]\n#[path = \"tests/model.rs\"]\nmod tests;\n\
         const TEXT: &str = \"#[path = \\\"tests/quoted.rs\\\"] mod quoted;\";\n",
    );
    workspace.write(&format!("{CRATE}/src/tests/quoted.rs"), "\n");
    let found = findings(&workspace);
    assert_eq!(found.len(), 2, "{found:#?}");
    for file in ["src/tests/chained.rs", "src/tests/quoted.rs"] {
        assert!(
            found
                .iter()
                .any(|f| f.starts_with(&format!("{CRATE}/{file}:"))),
            "{file}: {found:#?}"
        );
    }
}

/// A binary declared in the manifest is a root, a nested package is another crate's business,
/// and files outside test folders are not judged.
#[test]
fn test_file_reachability_declared_binaries_roots_nested_packages_and_production_files() {
    let mut workspace = FixtureWorkspace::new("reachability-binary");
    workspace.member(
        "tools/xtask",
        "[package]\nname = \"xtask\"\nedition = \"2024\"\n\n[[bin]]\nname = \"xtask\"\npath = \"src/entry.rs\"\n",
    );
    workspace.write(
        "tools/xtask/src/entry.rs",
        "#[cfg(test)]\n#[path = \"tests/entry.rs\"]\nmod tests;\n",
    );
    workspace.write("tools/xtask/src/tests/entry.rs", "\n");
    workspace.write("tools/xtask/src/orphan_production.rs", "\n");
    workspace.write(
        "tools/xtask/fixtures/nested/Cargo.toml",
        "[package]\nname = \"nested\"\n",
    );
    workspace.write("tools/xtask/fixtures/nested/src/tests/lost.rs", "\n");
    let report = check_test_file_reachability(workspace.root());
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
}

/// A trybuild case is compiled by trybuild as a crate of its own: a reached file naming it in a
/// `compile_fail` or `pass` call reaches it, a wildcard case reaches every file it matches, and
/// a case no reached file names stays unreachable.
#[test]
fn test_file_reachability_trybuild_cases_a_reached_file_names_are_reached() {
    let workspace = green_workspace("reachability-trybuild");
    workspace.write(
        &format!("{CRATE}/tests/trybuild.rs"),
        "#[test]\nfn ui() {\n    let t = trybuild::TestCases::new();\n    \
         t.compile_fail(\"tests/fail/closed_enum.rs\");\n    t.pass(\"tests/pass/*.rs\");\n}\n",
    );
    for case in [
        "tests/fail/closed_enum.rs",
        "tests/pass/one.rs",
        "tests/pass/two.rs",
        "tests/fail/unnamed.rs",
    ] {
        workspace.write(&format!("{CRATE}/{case}"), "fn main() {}\n");
    }
    let found = findings(&workspace);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].starts_with(&format!("{CRATE}/tests/fail/unnamed.rs:")),
        "{found:#?}"
    );
}

#[test]
fn test_file_reachability_lists_the_unreachable_files_with_their_package() {
    let mut workspace = green_workspace("reachability-listing");
    workspace.member("apps/api", &application_manifest("api", ""));
    workspace.write("apps/api/src/main.rs", "fn main() {}\n");
    workspace.write("apps/api/tests/common/helpers.rs", "\n");
    assert_eq!(
        unreachable_test_files(workspace.root()).unwrap(),
        vec![UnreachableTestFile {
            path: "apps/api/tests/common/helpers.rs".to_owned(),
            package: "api".to_owned(),
        }]
    );
}

#[test]
fn test_file_reachability_a_missing_member_folder_did_not_run() {
    let workspace = green_workspace("reachability-missing");
    std::fs::remove_dir_all(workspace.root().join(CRATE)).unwrap();
    let report = check_test_file_reachability(workspace.root());
    assert_eq!(report.exit_code, 2);
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("TEST-FILE-REACHABILITY: FAIL (did not run)")
    );
}

#[test]
fn test_file_reachability_this_checkout_passes() {
    let report = check_test_file_reachability(&crate::temporary_checkout::this_repository());
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
}
