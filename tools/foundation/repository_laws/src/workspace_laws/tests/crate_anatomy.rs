//! Tests for [`super`] — the manifest and source halves of the crate-anatomy law.

use super::*;
use crate::temporary_checkout::this_repository;
use crate::workspace_laws::fixture_workspace::{Dependency, FixtureWorkspace, normal};

const CRATE: &str = "crates/foundation/newtype_ids";

fn workspace(name: &str) -> FixtureWorkspace {
    let mut workspace = FixtureWorkspace::new(name);
    workspace.layout_crate(CRATE, 0, "any", &[]);
    workspace.layout_crate(
        "crates/foundation/time_source",
        1,
        "any",
        &[normal("newtype_ids")],
    );
    workspace
}

fn findings(workspace: &FixtureWorkspace) -> Vec<String> {
    crate_anatomy_outcome(workspace.root())
        .expect("the law runs")
        .findings
}

fn assert_finding(workspace: &FixtureWorkspace, needle: &str) {
    let found = findings(workspace);
    assert!(
        found.iter().any(|finding| finding.contains(needle)),
        "expected a finding containing {needle:?}, got {found:#?}"
    );
}

#[test]
fn crate_anatomy_a_green_library_passes_and_a_binary_is_exempt() {
    let mut workspace = workspace("anatomy-green");
    workspace.member("tools/staging/staging_fixtures", "[package]\nname = \"staging_fixtures\"\n\n[package.metadata.layout]\ncategory = \"tools/staging\"\ntier = 0\ntargets = \"any\"\n");
    workspace.write(
        "tools/staging/staging_fixtures/src/main.rs",
        "fn main() {}\n",
    );
    let report = check_crate_anatomy(workspace.root());
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert!(report.lines[0].contains("2 judged library crate(s)"));
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("1 judged binary crate(s) exempt: tools/staging/staging_fixtures"))
    );
}

#[test]
fn crate_anatomy_a_long_lib_rs_or_one_with_a_fn_is_a_finding() {
    let workspace = workspace("anatomy-lib-rs");
    let long = format!("pub mod prelude;\n{}", "//! line\n".repeat(80));
    workspace.write(&format!("{CRATE}/src/lib.rs"), &long);
    assert_finding(&workspace, "81 lines; lib.rs holds at most 80");
    workspace.write(
        &format!("{CRATE}/src/lib.rs"),
        "//! Ids.\n#[cfg(test)]\nmod tests;\npub mod prelude;\npub use prelude::{\n    A,\n    B,\n};\nfn helper() {}\n",
    );
    let found = findings(&workspace);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].contains("src/lib.rs:9: lib.rs holds only") && found[0].contains("fn helper"));
}

#[test]
fn crate_anatomy_a_missing_prelude_is_a_finding() {
    let workspace = workspace("anatomy-prelude");
    workspace.write(&format!("{CRATE}/src/lib.rs"), "//! Ids.\nmod ids;\n");
    assert_finding(&workspace, "no `pub mod prelude;`");
}

#[test]
fn crate_anatomy_a_fallible_api_needs_error_rs() {
    let workspace = workspace("anatomy-error");
    workspace.write(
        &format!("{CRATE}/src/ids.rs"),
        "pub fn parse(\n    text: Text,\n) -> Result<Id> {\n    todo!()\n}\n",
    );
    assert_finding(
        &workspace,
        "src/error.rs is missing; the public API is fallible (crates/foundation/newtype_ids/src/ids.rs:1)",
    );
    workspace.write(&format!("{CRATE}/src/error.rs"), "pub enum Error {}\n");
    assert_finding(&workspace, "never derives its Error with thiserror");
    assert_finding(&workspace, "never declares `pub type Result`");
    workspace.write(
        &format!("{CRATE}/src/error.rs"),
        "#[derive(Debug, thiserror::Error)]\npub enum Error {}\npub type Result<T> = std::result::Result<T, Error>;\n",
    );
    assert_eq!(findings(&workspace), Vec::<String>::new());
}

#[test]
fn crate_anatomy_anyhow_and_non_workspace_dependencies_are_findings() {
    let mut workspace = FixtureWorkspace::new("anatomy-dependencies");
    workspace.layout_crate(CRATE, 0, "any", &[normal("anyhow")]);
    assert_finding(&workspace, "a library crate never depends on anyhow");
    let manifest =
        std::fs::read_to_string(workspace.root().join(CRATE).join("Cargo.toml")).unwrap();
    let local = manifest
        .replace("anyhow = { workspace = true }", "serde = \"1\"")
        .replace("edition.workspace = true", "edition = \"2024\"")
        .replace(
            "[lints]\nworkspace = true",
            "[lints.rust]\nunsafe_code = \"forbid\"",
        );
    workspace.write(&format!("{CRATE}/Cargo.toml"), &local);
    let found = findings(&workspace);
    assert_eq!(found.len(), 3, "{found:#?}");
    assert!(found[0].contains("`edition` is set locally"));
    assert!(found[1].contains("lints come from the workspace"));
    assert!(found[2].contains("serde is not a workspace dependency"));
}

#[test]
fn crate_anatomy_a_forbidden_feature_or_a_shipped_dev_feature_is_a_finding() {
    let mut workspace = workspace("anatomy-features");
    let path = format!("{CRATE}/Cargo.toml");
    let manifest = std::fs::read_to_string(workspace.root().join(&path)).unwrap();
    workspace.write(
        &path,
        &format!("{manifest}\n[features]\nserde = []\ntest_fixtures = []\n"),
    );
    assert_finding(
        &workspace,
        "feature `serde` — the only features are test_fixtures and failpoints",
    );
    workspace.layout_crate(
        "crates/foundation/content_digest",
        1,
        "any",
        &[Dependency {
            package: "newtype_ids",
            table: "dev-dependencies",
        }],
    );
    let consumer = "crates/foundation/content_digest/Cargo.toml";
    let text = std::fs::read_to_string(workspace.root().join(consumer)).unwrap();
    workspace.write(
        consumer,
        &text.replace(
            "newtype_ids = { workspace = true }",
            "newtype_ids = { workspace = true, features = [\"test_fixtures\"] }",
        ),
    );
    assert_eq!(
        findings(&workspace).len(),
        1,
        "a dev-dependency may enable test_fixtures"
    );
    let text = std::fs::read_to_string(
        workspace
            .root()
            .join("crates/foundation/time_source/Cargo.toml"),
    )
    .unwrap();
    workspace.write(
        "crates/foundation/time_source/Cargo.toml",
        &text.replace(
            "newtype_ids = { workspace = true }",
            "newtype_ids = { workspace = true, features = [\"test_fixtures\"] }",
        ),
    );
    assert_finding(
        &workspace,
        "[dependencies] enables `newtype_ids/test_fixtures`; only a dev-dependency may",
    );
}

#[test]
fn crate_anatomy_a_primitive_public_id_is_a_finding_outside_generated_and_wasm_bindgen() {
    let workspace = workspace("anatomy-ids");
    workspace.write(&format!("{CRATE}/src/user.rs"), "pub struct User {\n    pub user_id: String,\n    pub name: String,\n}\npub fn load(store: &Store, user_id: &str) {}\npub fn typed(user_id: UserId) {}\n");
    let found = findings(&workspace);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(found[0].contains("src/user.rs:2: a public id field is a primitive"));
    assert!(found[1].contains("src/user.rs:5: a pub fn takes a primitive id parameter"));
    std::fs::remove_file(workspace.root().join(CRATE).join("src/user.rs")).unwrap();
    workspace.write(
        &format!("{CRATE}/src/generated/user.rs"),
        "pub struct User {\n    pub user_id: String,\n}\n",
    );
    workspace.write(
        &format!("{CRATE}/src/bindings.rs"),
        "#[wasm_bindgen]\nimpl Handle {\n    pub fn select(&self, unit_id: u32) {}\n}\n",
    );
    assert_eq!(findings(&workspace), Vec::<String>::new());
}

#[test]
fn crate_anatomy_a_workspace_reexport_outside_the_prelude_is_a_finding() {
    let workspace = workspace("anatomy-reexport");
    let time_source = "crates/foundation/time_source/src";
    workspace.write(
        &format!("{time_source}/clock.rs"),
        "pub use newtype_ids::EventId;\npub use std::time::Duration;\n",
    );
    let found = findings(&workspace);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].contains(
        "time_source/src/clock.rs:1: `pub use newtype_ids::…` outside the prelude module"
    ));
    std::fs::remove_file(workspace.root().join(time_source).join("clock.rs")).unwrap();
    workspace.write(
        &format!("{time_source}/prelude.rs"),
        "pub use newtype_ids::EventId;\n",
    );
    assert_eq!(findings(&workspace), Vec::<String>::new());
}

#[test]
fn crate_anatomy_a_library_without_lib_rs_did_not_run() {
    let workspace = workspace("anatomy-missing");
    std::fs::remove_file(workspace.root().join(CRATE).join("src/lib.rs")).unwrap();
    let mut manifest =
        std::fs::read_to_string(workspace.root().join(CRATE).join("Cargo.toml")).unwrap();
    manifest.push_str("\n[lib]\npath = \"src/lib.rs\"\n");
    workspace.write(&format!("{CRATE}/Cargo.toml"), &manifest);
    assert_eq!(check_crate_anatomy(workspace.root()).exit_code, 2);
}

#[test]
fn crate_anatomy_this_checkout_passes() {
    let report = check_crate_anatomy(&this_repository());
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
}
