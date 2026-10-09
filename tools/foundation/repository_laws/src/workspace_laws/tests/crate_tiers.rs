//! Tests for [`super`] — the application rule and the external-crate firewalls over fixture
//! workspaces, and the law over this checkout.

use super::*;
use crate::temporary_checkout::this_repository;
use crate::workspace_laws::fixture_workspace::{Dependency, green_workspace, normal};

/// The fixture configuration with the application of [`green_workspace`], `api_server`, listed.
const SERVER_APPLICATION: &CrateTierConfiguration<'static> = &CrateTierConfiguration {
    application_packages: &["api_server"],
};

/// The application packages of this repository, as `cargo xtask verify crate-tiers` passes them.
const THIS_REPOSITORY: &CrateTierConfiguration<'static> = &CrateTierConfiguration {
    application_packages: &[
        "api_server",
        "frontend_application",
        "offline_service_worker",
        "game_server_host_agent",
        "ticketboard_desktop",
    ],
};

/// The dependency tables an edge onto an application may hide in.
const EVERY_TABLE: &[&str] = &[
    "dependencies",
    "dev-dependencies",
    "build-dependencies",
    "target.'cfg(target_arch = \"wasm32\")'.dependencies",
];

/// The 1-based line of the one dependency a `layout_crate` fixture manifest declares.
const FIRST_DEPENDENCY_LINE: usize = 16;

fn findings(workspace: &crate::workspace_laws::fixture_workspace::FixtureWorkspace) -> Vec<String> {
    crate_tier_outcome(workspace.root(), SERVER_APPLICATION)
        .expect("the law runs")
        .findings
}

#[test]
fn crate_tiers_a_green_workspace_passes() {
    let workspace = green_workspace("tiers-green");
    let report = check_crate_tiers(workspace.root(), SERVER_APPLICATION);
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("CRATE-TIERS: PASS")
    );
}

#[test]
fn crate_tiers_an_edge_onto_an_application_in_any_table_is_a_finding() {
    for &table in EVERY_TABLE {
        let mut workspace = green_workspace("tiers-application-table");
        workspace.layout_crate(
            "crates/foundation/deterministic_random",
            0,
            "any",
            &[Dependency {
                package: "api_server",
                table,
            }],
        );
        let expected = format!(
            "application edge: crates/foundation/deterministic_random/Cargo.toml:\
             {FIRST_DEPENDENCY_LINE}: [{table}] depends on the application api_server — no \
             member depends on an application package, in any table"
        );
        assert_eq!(findings(&workspace), vec![expected], "{table}");
    }
}

#[test]
fn crate_tiers_an_application_package_that_is_no_member_is_a_finding() {
    let workspace = green_workspace("tiers-application-absent");
    let configuration = CrateTierConfiguration {
        application_packages: &["api_server", "retired_application"],
    };
    let report = check_crate_tiers(workspace.root(), &configuration);
    assert_eq!(report.exit_code, 1, "{}", report.lines.join("\n"));
    assert!(
        report.lines.contains(
            &"FAIL: application edge: application package `retired_application` is no \
              workspace member — the application list names members only"
                .to_string()
        ),
        "{}",
        report.lines.join("\n")
    );
}

#[test]
fn crate_tiers_wgpu_in_a_cpu_crate_and_sqlx_outside_api_are_firewall_findings() {
    let mut workspace = green_workspace("tiers-firewall");
    workspace.layout_crate("crates/geometry/camera_math", 0, "any", &[normal("wgpu")]);
    workspace.layout_crate("crates/mission/mission_store", 0, "any", &[normal("sqlx")]);
    workspace.layout_crate("crates/graphics/gpu_device", 0, "any", &[normal("wgpu")]);
    workspace.layout_crate("crates/api/api_database", 0, "any", &[normal("sqlx")]);
    let found = findings(&workspace);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(
        found[0].starts_with("firewall: crates/geometry/camera_math/Cargo.toml")
            && found[0].contains("wgpu")
    );
    assert!(
        found[1].starts_with("firewall: crates/mission/mission_store/Cargo.toml")
            && found[1].contains("sqlx and axum")
    );
}

#[test]
fn crate_tiers_this_checkout_passes() {
    let report = check_crate_tiers(&this_repository(), THIS_REPOSITORY);
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
}
