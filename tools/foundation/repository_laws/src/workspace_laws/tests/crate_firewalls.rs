//! Tests for [`super`] — the `#[wasm_bindgen]` export scan over the workspace members, and the
//! rendering-stack clause of the offline service worker and the API crates.

use super::*;
use crate::temporary_checkout::this_repository;
use crate::workspace_laws::crate_layout::is_judged;
use crate::workspace_laws::fixture_workspace::{
    Dependency, FixtureWorkspace, application_manifest,
};
use crate::workspace_members::read_workspace_members;

/// A workspace holding the three exporting members and one library crate, each with sources.
fn exporting_workspace(name: &str) -> FixtureWorkspace {
    let mut workspace = FixtureWorkspace::new(name);
    for (path, package) in [
        (
            "crates/frontend/shell/frontend_application",
            "frontend_application",
        ),
        (
            "crates/frontend/shell/offline_service_worker",
            "offline_service_worker",
        ),
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
        "crates/frontend/shell/frontend_application/src/main.rs",
        "#[wasm_bindgen(start)]\npub fn main() {}\n",
    );
    workspace.write(
        "crates/frontend/shell/offline_service_worker/src/events.rs",
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

/// The tables an edge onto the rendering stack may hide in.
const EVERY_TABLE: &[&str] = &[
    "dependencies",
    "dev-dependencies",
    "build-dependencies",
    "target.'cfg(target_arch = \"wasm32\")'.dependencies",
];

/// A workspace of the worker, the app, one API crate and one crate of each rendering-stack
/// category, plus `from` depending on `package` from `table`.
fn rendering_stack_workspace(from: &str, package: &str, table: &str) -> FixtureWorkspace {
    let name = format!(
        "rendering-stack-{}-{package}-{}",
        from.replace('/', "-"),
        table.len()
    );
    let mut workspace = FixtureWorkspace::new(&name);
    for path in [
        "crates/frontend/shell/offline_service_worker",
        "crates/frontend/shell/frontend_application",
        "crates/api/api_state",
        "crates/contracts/offline_cache_policy",
        "crates/graphics/gpu_frame",
        "crates/map_rendering/map_renderer",
        "crates/paper_doll/paper_doll_renderer",
        "crates/streaming/chunk_scheduler",
    ] {
        let dependencies = if path == from {
            vec![Dependency { package, table }]
        } else {
            Vec::new()
        };
        workspace.layout_crate(path, 0, "any", &dependencies);
    }
    workspace
}

/// The rendering-stack clause's findings over `workspace`.
fn rendering_stack(workspace: &FixtureWorkspace) -> Vec<String> {
    let members = read_workspace_members(workspace.root()).unwrap();
    let judged: Vec<&WorkspaceMember> = members.iter().filter(|m| is_judged(m)).collect();
    rendering_stack_findings(&members, &judged)
}

#[test]
fn crate_firewalls_the_worker_reaches_no_rendering_stack_crate_or_wgpu_in_any_table() {
    for &table in EVERY_TABLE {
        for package in [
            "gpu_frame",
            "map_renderer",
            "paper_doll_renderer",
            "chunk_scheduler",
            "wgpu",
            "wgpu-types",
        ] {
            let workspace = rendering_stack_workspace(
                "crates/frontend/shell/offline_service_worker",
                package,
                table,
            );
            assert_eq!(
                rendering_stack(&workspace),
                vec![format!(
                    "rule 6: crates/frontend/shell/offline_service_worker/Cargo.toml:16: \
                     [{table}] {package} — the offline service worker and the API crates link no \
                     graphics, map rendering, paper doll or streaming crate and no wgpu, in any \
                     table"
                )],
                "{package} [{table}]"
            );
        }
    }
}

#[test]
fn crate_firewalls_an_api_crate_reaches_no_rendering_stack_crate_even_from_a_dev_table() {
    let workspace =
        rendering_stack_workspace("crates/api/api_state", "map_renderer", "dev-dependencies");
    let found = rendering_stack(&workspace);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].starts_with(
            "rule 6: crates/api/api_state/Cargo.toml:16: [dev-dependencies] map_renderer — "
        ),
        "{found:#?}"
    );
}

/// The clause binds the worker and the API crates only: the worker's cache policy passes, and
/// the app shell beside it draws the map.
#[test]
fn crate_firewalls_the_rendering_stack_clause_binds_only_the_worker_and_the_api_crates() {
    for (from, package) in [
        (
            "crates/frontend/shell/offline_service_worker",
            "offline_cache_policy",
        ),
        ("crates/frontend/shell/frontend_application", "gpu_frame"),
        ("crates/frontend/shell/frontend_application", "map_renderer"),
    ] {
        let workspace = rendering_stack_workspace(from, package, "dependencies");
        assert_eq!(rendering_stack(&workspace), Vec::<String>::new(), "{from}");
    }
}

#[test]
fn crate_firewalls_this_checkout_keeps_the_rendering_stack_clause() {
    let members = read_workspace_members(&this_repository()).unwrap();
    assert!(
        members
            .iter()
            .any(|m| m.package_name == OFFLINE_SERVICE_WORKER_PACKAGE),
        "the clause binds a member"
    );
    let judged: Vec<&WorkspaceMember> = members.iter().filter(|m| is_judged(m)).collect();
    assert_eq!(
        rendering_stack_findings(&members, &judged),
        Vec::<String>::new()
    );
}
