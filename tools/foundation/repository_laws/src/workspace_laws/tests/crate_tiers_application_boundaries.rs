//! Tests for [`super`] — rule 7 (no member depends on an application package, in any table), the
//! fleet and shell categories, and the retired dependency deny-lists of the website applications,
//! each row of which the crate-tier law now catches over fixture workspaces.

use super::tests::{CONFIGURATION, assert_one_finding};
use super::*;
use crate::workspace_laws::fixture_workspace::{
    Dependency, FixtureWorkspace, application_manifest, green_workspace, normal,
};

/// The fixture configuration with the application of [`green_workspace`], `api_server`, listed.
const SERVER_APPLICATION: &CrateTierConfiguration<'static> = &CrateTierConfiguration {
    application_packages: &["api_server"],
    ..*CONFIGURATION
};

/// The crate-tier findings over `workspace` with [`SERVER_APPLICATION`].
fn findings(workspace: &FixtureWorkspace) -> Vec<String> {
    crate_tier_outcome(workspace.root(), SERVER_APPLICATION)
        .expect("the law runs")
        .findings
}

/// The dependency tables an edge onto an application may hide in.
const EVERY_TABLE: &[&str] = &[
    "dependencies",
    "dev-dependencies",
    "build-dependencies",
    "target.'cfg(target_arch = \"wasm32\")'.dependencies",
];

/// The 1-based line of the one dependency a [`FixtureWorkspace::layout_crate`] manifest declares.
const FIRST_DEPENDENCY_LINE: usize = 16;

/// The rule 7 finding of `crate_path` depending on `application` from `table`.
fn rule_7(crate_path: &str, table: &str, application: &str) -> String {
    format!(
        "rule 7: {crate_path}/Cargo.toml:{FIRST_DEPENDENCY_LINE}: [{table}] depends on the \
         application {application} — no member depends on an application package, in any table"
    )
}

#[test]
fn crate_tiers_rule_7_an_edge_onto_an_application_in_any_table_is_a_finding() {
    for &table in EVERY_TABLE {
        let mut workspace = green_workspace("tiers-rule-7-table");
        workspace.layout_crate(
            "crates/foundation/deterministic_random",
            0,
            "any",
            &[Dependency {
                package: "api_server",
                table,
            }],
        );
        let expected = rule_7(
            "crates/foundation/deterministic_random",
            table,
            "api_server",
        );
        let found = findings(&workspace);
        assert!(found.contains(&expected), "{table}: {found:#?}");
        if table == "dev-dependencies" {
            assert_eq!(found, vec![expected], "a dev edge breaks rule 7 alone");
        }
    }
}

#[test]
fn crate_tiers_rule_7_reads_the_real_package_of_a_renamed_edge_and_spares_a_self_edge() {
    let mut workspace = green_workspace("tiers-rule-7-renamed");
    workspace.member(
        "crates/foundation/deterministic_random",
        "[package]\nname = \"deterministic_random\"\n\n[package.metadata.layout]\n\
         category = \"crates/foundation\"\ntier = 0\ntargets = \"any\"\n\n\
         [dev-dependencies]\nserver = { package = \"api_server\", path = \"../../api/api_server\" }\n",
    );
    let found = findings(&workspace);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].starts_with(
            "rule 7: crates/foundation/deterministic_random/Cargo.toml:10: [dev-dependencies] \
             depends on the application api_server"
        ),
        "{found:#?}"
    );
    let mut workspace = FixtureWorkspace::new("tiers-rule-7-self");
    workspace.member(
        "crates/api/api_server",
        "[package]\nname = \"api_server\"\n\n[package.metadata.layout]\ncategory = \
         \"crates/api\"\ntier = 0\ntargets = \"any\"\n\n[features]\nfailpoints = []\n\n\
         [dev-dependencies]\napi_server = { path = \".\", features = [\"failpoints\"] }\n",
    );
    workspace.write("crates/api/api_server/src/lib.rs", "//! The server.\n");
    assert_eq!(findings(&workspace), Vec::<String>::new());
}

#[test]
fn crate_tiers_rule_7_a_tool_binary_depending_on_an_application_is_a_finding() {
    let mut workspace = green_workspace("tiers-rule-7-binary");
    workspace.member(
        "tools/xtask",
        &application_manifest(
            "xtask",
            "api_server = { path = \"../../crates/api/api_server\" }\n",
        ),
    );
    assert_eq!(
        findings(&workspace),
        vec![
            "rule 7: tools/xtask/Cargo.toml:6: [dependencies] depends on the application \
             api_server — no member depends on an application package, in any table"
                .to_string()
        ]
    );
}

#[test]
fn crate_tiers_rule_7_an_application_package_that_is_no_member_is_a_finding() {
    let workspace = green_workspace("tiers-rule-7-absent");
    let configuration = CrateTierConfiguration {
        application_packages: &["api_server", "retired_application"],
        ..*SERVER_APPLICATION
    };
    let report = check_crate_tiers(workspace.root(), &configuration);
    assert_eq!(report.exit_code, 1, "{}", report.lines.join("\n"));
    assert!(
        report.lines.contains(
            &"FAIL: rule 7: application package `retired_application` is no workspace member — \
              the application list names members only"
                .to_string()
        ),
        "{}",
        report.lines.join("\n")
    );
}

/// A crate under `apps/` is judged like any other member: outside the layout folders and with no
/// layout table it is a rule 2 finding, never exempt.
#[test]
fn crate_tiers_a_crate_under_apps_is_rule_2() {
    let mut workspace = green_workspace("tiers-apps-crate");
    workspace.member("apps/server", &application_manifest("server", ""));
    let found = findings(&workspace);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].starts_with(
            "rule 2: apps/server is a member outside the judged set — move it to \
             crates/<category>/<name> or tools/<category>/<name>"
        ) && found[0]
            .ends_with("only the tool binaries (tools/xtask, tools/developer_tools) stay outside"),
        "{found:#?}"
    );
}

/// A manifest in `crates/fleet` that no member glob reaches is a stray manifest of rule 1.
#[test]
fn crate_tiers_a_fleet_crate_without_its_member_glob_is_rule_1() {
    let mut workspace = green_workspace("tiers-fleet-glob");
    workspace.layout_crate("crates/contracts/fleet_wire_contract", 0, "any", &[]);
    workspace.layout_crate(
        "crates/fleet/game_server_host_agent",
        1,
        "any",
        &[normal("fleet_wire_contract")],
    );
    let globs = "[workspace]\nresolver = \"3\"\nmembers = [\n    \"crates/api/*\",\n    \
                 \"crates/contracts/*\",\n    \"crates/fleet/*\",\n    \"crates/foundation/*\",\n    \
                 \"crates/mission/*\",\n    \"tools/foundation/*\",\n]\n\n[workspace.package]\n\
                 edition = \"2024\"\n";
    workspace.write("Cargo.toml", globs);
    assert_eq!(findings(&workspace), Vec::<String>::new());
    workspace.write(
        "Cargo.toml",
        &globs.replace("    \"crates/fleet/*\",\n", ""),
    );
    assert_eq!(
        findings(&workspace),
        vec![
            "rule 1: crates/fleet/game_server_host_agent/Cargo.toml is not a workspace member — \
             add the folder to the root [workspace] members"
                .to_string()
        ]
    );
}

/// The fleet row of the matrix: foundation crates built for every target and contracts crates,
/// nothing else.
#[test]
fn crate_tiers_a_fleet_crate_reaches_only_every_target_foundation_and_contracts() {
    let mut workspace = green_workspace("tiers-fleet-allowed");
    workspace.layout_crate("crates/contracts/fleet_wire_contract", 0, "any", &[]);
    workspace.layout_crate(
        "crates/fleet/game_server_host_agent",
        2,
        "any",
        &[normal("fleet_wire_contract"), normal("time_source")],
    );
    assert_eq!(findings(&workspace), Vec::<String>::new());
    for (path, targets) in [
        ("crates/foundation/browser_platform", "wasm32"),
        ("crates/mission/mission_payload", "any"),
        ("crates/api/api_state", "any"),
        ("tools/foundation/process_runner", "any"),
        ("crates/geometry/geometry_primitives", "any"),
    ] {
        let mut workspace = green_workspace("tiers-fleet-refused");
        workspace.layout_crate(path, 0, targets, &[]);
        let package = path.rsplit('/').next().unwrap();
        workspace.layout_crate(
            "crates/fleet/game_server_host_agent",
            1,
            "any",
            &[normal(package)],
        );
        assert_one_finding(
            &workspace,
            &format!("crates/fleet may not depend on {path}"),
        );
    }
}

/// No row of the matrix admits the fleet: the frontend's explicit allow-list leaves it out like
/// every other row.
#[test]
fn crate_tiers_no_category_reaches_the_fleet() {
    for from in [
        "crates/frontend/pages/operations_pages",
        "crates/frontend/shell/frontend_application",
        "crates/api/api_state",
        "tools/commands/deployment",
        "crates/mission/mission_payload",
        "crates/map_rendering/map_renderer",
        "crates/mission_editing/mission_editing_commands",
        "crates/streaming/chunk_scheduler",
        "crates/contracts/fleet_probe_contract",
        "crates/foundation/fleet_probe",
    ] {
        let mut workspace = green_workspace("tiers-onto-fleet");
        workspace.layout_crate("crates/fleet/game_server_host_agent", 0, "any", &[]);
        workspace.layout_crate(from, 1, "any", &[normal("game_server_host_agent")]);
        let category = from.rsplit_once('/').unwrap().0;
        assert_one_finding(
            &workspace,
            &format!(
                "rule 5: {from}/Cargo.toml:{FIRST_DEPENDENCY_LINE}: {category} may not depend on \
                 crates/fleet/game_server_host_agent (crates/fleet)"
            ),
        );
    }
}

/// `crates/frontend/shell` is a frontend layer: its crates read the pages and foundation crates
/// and carry leptos and the browser crates like any other frontend crate.
#[test]
fn crate_tiers_the_shell_category_is_a_frontend_layer() {
    let mut workspace = green_workspace("tiers-shell-layer");
    workspace.layout_crate("crates/frontend/pages/account_pages", 0, "any", &[]);
    workspace.layout_crate(
        "crates/frontend/shell/frontend_application",
        1,
        "any",
        &[
            normal("account_pages"),
            normal("leptos"),
            Dependency {
                package: "wasm-bindgen",
                table: "target.'cfg(target_arch = \"wasm32\")'.dependencies",
            },
        ],
    );
    assert_eq!(findings(&workspace), Vec::<String>::new());
}

/// The base of one retired deny-list row: the three website applications and the eight GPU crates
/// the deny-lists named but `planted` (which the row adds with its edge), each a crate of its
/// category at tier 0, with every application listed.
fn deny_list_workspace(planted: &str) -> (FixtureWorkspace, CrateTierConfiguration<'static>) {
    let mut workspace = FixtureWorkspace::new("tiers-deny-list-row");
    for path in DENY_LIST_CRATES.iter().filter(|path| **path != planted) {
        workspace.layout_crate(path, 0, "any", &[]);
    }
    let configuration = CrateTierConfiguration {
        manifest_sweep_roots: CONFIGURATION.manifest_sweep_roots,
        application_packages: &[
            "api_server",
            "frontend_application",
            "offline_service_worker",
        ],
    };
    (workspace, configuration)
}

/// Every crate a retired deny-list row named, on either side.
const DENY_LIST_CRATES: &[&str] = &[
    "crates/api/api_server",
    "crates/frontend/shell/frontend_application",
    "crates/frontend/shell/offline_service_worker",
    "crates/graphics/gpu_device",
    "crates/graphics/gpu_frame",
    "crates/graphics/renderer_core",
    "crates/map_rendering/map_renderer",
    "crates/map_rendering/symbology_layers_gpu",
    "crates/map_rendering/world_layers_gpu",
    "crates/map_rendering/map_render_diagnostics",
    "crates/paper_doll/paper_doll_renderer",
];

/// The rendering-stack finding of `crate_path` depending on `package` from `table`.
fn rendering_stack(crate_path: &str, table: &str, package: &str) -> String {
    format!(
        "rule 6: {crate_path}/Cargo.toml:{FIRST_DEPENDENCY_LINE}: [{table}] {package} — the \
         offline service worker and the API crates link no graphics, map rendering, paper doll or \
         streaming crate and no wgpu, in any table"
    )
}

/// Every row of the retired deny-lists of the frontend, the server and the offline service worker,
/// planted as a dev-dependency (the table the category matrix does not read), is caught by rule 7
/// or by the rendering-stack clause; planted as a normal dependency it is caught as well, the
/// category matrix (rule 5) adding its own finding where the categories forbid the edge. The
/// worker and the app naming each other is also a shell crate order edge of the frontend-layering
/// law (`frontend_layering_crate_edges.rs`).
#[test]
fn crate_tiers_every_retired_deny_list_row_is_caught() {
    const API: &str = "crates/api/api_server";
    const APP: &str = "crates/frontend/shell/frontend_application";
    const WORKER: &str = "crates/frontend/shell/offline_service_worker";
    const GPU: &[&str] = &[
        "gpu_device",
        "gpu_frame",
        "renderer_core",
        "map_renderer",
        "symbology_layers_gpu",
        "world_layers_gpu",
        "map_render_diagnostics",
        "paper_doll_renderer",
    ];
    let mut rows: Vec<(&str, &str)> = vec![
        (APP, "api_server"),
        (API, "frontend_application"),
        (WORKER, "api_server"),
        (WORKER, "frontend_application"),
    ];
    rows.extend(GPU.iter().map(|package| (API, *package)));
    rows.extend(GPU.iter().map(|package| (WORKER, *package)));
    assert_eq!(rows.len(), 20, "1 frontend, 9 server and 10 worker entries");
    for (from, package) in rows {
        let expected: fn(&str, &str, &str) -> String = if GPU.contains(&package) {
            rendering_stack
        } else {
            rule_7
        };
        for (table, tier) in [("dev-dependencies", 0), ("dependencies", 1)] {
            let (mut workspace, configuration) = deny_list_workspace(from);
            workspace.layout_crate(from, tier, "any", &[Dependency { package, table }]);
            let found = crate_tier_outcome(workspace.root(), &configuration)
                .expect("the law runs")
                .findings;
            let red = expected(from, table, package);
            assert!(
                found.contains(&red),
                "{from} -> {package} [{table}]: {found:#?}"
            );
            if table == "dev-dependencies" {
                assert_eq!(found, vec![red], "{from} -> {package}");
            }
        }
    }
}
