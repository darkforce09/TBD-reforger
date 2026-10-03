//! Tests for [`super`] — the `#[wasm_bindgen]` export scan over the workspace members.

use super::*;
use crate::temporary_checkout::this_repository;
use crate::workspace_laws::fixture_workspace::{FixtureWorkspace, application_manifest};
use crate::workspace_members::read_workspace_members;

/// A workspace holding the three exporting members and one library crate, each with sources.
fn exporting_workspace(name: &str) -> FixtureWorkspace {
    let mut workspace = FixtureWorkspace::new(name);
    for (path, package) in [
        ("apps/frontend", "frontend"),
        ("apps/offline_service_worker", "offline_service_worker"),
        ("crates/foundation/browser_platform", "browser_platform"),
        ("crates/map_rendering/map_renderer", "map_renderer"),
    ] {
        workspace.member(path, &application_manifest(package, ""));
        workspace.write(&format!("{path}/src/lib.rs"), "pub fn start() {}\n");
    }
    workspace
}

/// The scan's findings over `workspace`.
fn scan(workspace: &FixtureWorkspace) -> Vec<String> {
    let members = read_workspace_members(workspace.root()).unwrap();
    wasm_bindgen_attribute_findings(workspace.root(), &members).unwrap()
}

#[test]
fn crate_firewalls_the_three_exporting_members_may_carry_the_attribute() {
    let workspace = exporting_workspace("wasm-bindgen-allowed");
    workspace.write(
        "apps/frontend/src/main.rs",
        "#[wasm_bindgen(start)]\npub fn main() {}\n",
    );
    workspace.write(
        "apps/offline_service_worker/src/events.rs",
        "    #[wasm_bindgen]\n    pub fn on_fetch() {}\n",
    );
    workspace.write(
        "crates/foundation/browser_platform/src/console.rs",
        "#[wasm_bindgen::prelude::wasm_bindgen]\nextern \"C\" {}\n",
    );
    assert_eq!(scan(&workspace), Vec::<String>::new());
}

#[test]
fn crate_firewalls_an_attribute_in_any_other_member_is_rule_6() {
    let workspace = exporting_workspace("wasm-bindgen-breach");
    workspace.write(
        "crates/map_rendering/map_renderer/src/exports.rs",
        concat!(
            "#[wasm_bindgen]\npub struct Engine;\n",
            "#[ wasm_bindgen::prelude::wasm_bindgen ]\nimpl Engine {}\n",
            "#[cfg_attr(target_arch = \"wasm32\", wasm_bindgen)]\npub fn boot() {}\n",
        ),
    );
    let finding = |line: usize, attribute: &str| {
        format!(
            "rule 6: crates/map_rendering/map_renderer/src/exports.rs:{line}: #[wasm_bindgen] \
             exports only from the frontend, browser_platform and the offline service worker: \
             {attribute}"
        )
    };
    assert_eq!(
        scan(&workspace),
        [
            finding(1, "#[wasm_bindgen]"),
            finding(3, "#[ wasm_bindgen::prelude::wasm_bindgen ]"),
            finding(5, "#[cfg_attr(target_arch = \"wasm32\", wasm_bindgen)]"),
        ]
    );
}

#[test]
fn crate_firewalls_a_mention_in_a_comment_or_string_is_not_an_export() {
    let workspace = exporting_workspace("wasm-bindgen-mentions");
    workspace.write(
        "crates/map_rendering/map_renderer/src/notes.rs",
        concat!(
            "//! No #[wasm_bindgen] here: the frontend exports.\n",
            "const FIXTURE: &str = \"#[wasm_bindgen]\\nimpl Handle {}\";\n",
            "pub fn wasm_bindgen_free() {}\n",
        ),
    );
    assert_eq!(scan(&workspace), Vec::<String>::new());
}

#[test]
fn crate_firewalls_a_workspace_without_rust_sources_is_not_a_clean_scan() {
    let mut workspace = FixtureWorkspace::new("wasm-bindgen-empty");
    workspace.member(
        "crates/map_rendering/map_renderer",
        &application_manifest("map_renderer", ""),
    );
    let found = scan(&workspace);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].contains("walked 0 .rs file(s)"), "{found:#?}");
}

#[test]
fn crate_firewalls_this_checkout_exports_only_from_the_three_members() {
    let root = this_repository();
    let members = read_workspace_members(&root).unwrap();
    let found = wasm_bindgen_attribute_findings(&root, &members).unwrap();
    assert_eq!(found, Vec::<String>::new());
}
