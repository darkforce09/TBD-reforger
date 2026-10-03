//! Tests for [`super`] — rules 1–8 of the crate-tier law over fixture workspaces.

use super::*;
use crate::temporary_checkout::this_repository;
use crate::workspace_laws::fixture_workspace::{
    Dependency, FixtureWorkspace, application_manifest, normal,
};

const SWEEP: &[&str] = &["apps", "crates", "tools", "legacy"];

/// A green workspace: two foundation crates, a mission crate, a tool and an application.
fn green_workspace(name: &str) -> FixtureWorkspace {
    let mut workspace = FixtureWorkspace::new(name);
    workspace.layout_crate("crates/foundation/newtype_ids", 0, "any", &[]);
    workspace.layout_crate(
        "crates/foundation/time_source",
        1,
        "any",
        &[normal("newtype_ids")],
    );
    workspace.layout_crate(
        "crates/mission/mission_model",
        2,
        "any",
        &[normal("time_source")],
    );
    workspace.layout_crate("tools/foundation/repository_layout", 0, "any", &[]);
    workspace.member(
        "apps/api",
        &application_manifest(
            "api",
            "mission_model = { path = \"../../crates/mission/mission_model\" }\n",
        ),
    );
    workspace
}

fn findings(workspace: &FixtureWorkspace) -> Vec<String> {
    crate_tier_outcome(workspace.root(), SWEEP)
        .expect("the law runs")
        .findings
}

fn assert_one_finding(workspace: &FixtureWorkspace, needle: &str) {
    let found = findings(workspace);
    assert!(
        found.iter().any(|finding| finding.contains(needle)),
        "expected a finding containing {needle:?}, got {found:#?}"
    );
}

#[test]
fn crate_tiers_a_green_workspace_passes_and_notes_the_unjudged_members() {
    let workspace = green_workspace("tiers-green");
    let report = check_crate_tiers(workspace.root(), SWEEP);
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("CRATE-TIERS: PASS")
    );
    assert!(
        report
            .lines
            .iter()
            .any(|l| l == "note: 1 member(s) outside the judged set: apps/api")
    );
    assert!(report.lines[0].contains("5 workspace member(s), 4 judged"));
}

#[test]
fn crate_tiers_a_wrong_declared_tier_is_rule_4() {
    let mut workspace = green_workspace("tiers-wrong-tier");
    workspace.layout_crate(
        "crates/mission/mission_payload",
        3,
        "any",
        &[normal("mission_model")],
    );
    let found = findings(&workspace);
    assert!(
        found.is_empty(),
        "tier 3 over a tier-2 dependency is right: {found:#?}"
    );
    let mut workspace = green_workspace("tiers-wrong-tier-red");
    workspace.layout_crate(
        "crates/mission/mission_payload",
        2,
        "any",
        &[normal("mission_model")],
    );
    assert_one_finding(
        &workspace,
        "declares tier 2; its dependencies make it tier 3",
    );
}

#[test]
fn crate_tiers_an_edge_to_an_equal_or_higher_tier_is_an_upward_edge() {
    let mut workspace = FixtureWorkspace::new("tiers-upward");
    workspace.layout_crate("crates/foundation/content_digest", 1, "any", &[]);
    workspace.layout_crate(
        "crates/foundation/http_url_guard",
        1,
        "any",
        &[normal("content_digest")],
    );
    assert_one_finding(
        &workspace,
        "tier 1 depends on crates/foundation/content_digest at tier 1; edges point strictly down",
    );
}

#[test]
fn crate_tiers_a_forbidden_category_edge_is_rule_5() {
    let mut workspace = green_workspace("tiers-category");
    workspace.layout_crate(
        "crates/contracts/fleet_wire_contract",
        3,
        "any",
        &[normal("mission_model")],
    );
    assert_one_finding(
        &workspace,
        "rule 5: crates/contracts/fleet_wire_contract/Cargo.toml:",
    );
    assert_one_finding(
        &workspace,
        "crates/contracts may not depend on crates/mission/mission_model",
    );
    let mut workspace = green_workspace("tiers-category-tool");
    workspace.layout_crate("crates/api/api_state", 0, "any", &[]);
    workspace.layout_crate(
        "tools/commands/deployment",
        1,
        "any",
        &[normal("api_state")],
    );
    assert_one_finding(
        &workspace,
        "tools/commands may not depend on crates/api/api_state",
    );
    workspace.layout_crate(
        "tools/staging/staging_fixtures",
        1,
        "any",
        &[normal("api_state")],
    );
    let found = findings(&workspace);
    assert!(
        !found.iter().any(|f| f.contains("staging_fixtures")),
        "{found:#?}"
    );
}

#[test]
fn crate_tiers_wgpu_in_a_cpu_crate_and_sqlx_outside_api_are_rule_6() {
    let mut workspace = green_workspace("tiers-firewall");
    workspace.layout_crate("crates/geometry/camera_math", 0, "any", &[normal("wgpu")]);
    workspace.layout_crate("crates/mission/mission_store", 0, "any", &[normal("sqlx")]);
    workspace.layout_crate("crates/graphics/gpu_device", 0, "any", &[normal("wgpu")]);
    workspace.layout_crate("crates/api/api_database", 0, "any", &[normal("sqlx")]);
    let found = findings(&workspace);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(
        found[0].contains("crates/geometry/camera_math/Cargo.toml") && found[0].contains("wgpu")
    );
    assert!(
        found[1].contains("crates/mission/mission_store/Cargo.toml")
            && found[1].contains("sqlx and axum")
    );
}

#[test]
fn crate_tiers_axum_in_a_browser_testing_or_staging_harness_server_passes() {
    let mut workspace = green_workspace("tiers-harness-axum");
    workspace.layout_crate(
        "tools/browser_testing/browser_gate_suites",
        0,
        "any",
        &[normal("axum")],
    );
    workspace.layout_crate(
        "tools/staging/acknowledgement_dropping_relay",
        0,
        "any",
        &[normal("axum")],
    );
    let found = findings(&workspace);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn crate_tiers_axum_in_any_other_tools_category_is_rule_6() {
    let mut workspace = green_workspace("tiers-tools-axum");
    workspace.layout_crate("tools/commands/deployment", 0, "any", &[normal("axum")]);
    let found = findings(&workspace);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].contains("tools/commands/deployment/Cargo.toml")
            && found[0].contains("axum — sqlx and axum live only in api crates"),
        "{found:#?}"
    );
}

#[test]
fn crate_tiers_sqlx_in_a_browser_testing_or_staging_crate_is_rule_6() {
    let mut workspace = green_workspace("tiers-harness-sqlx");
    workspace.layout_crate(
        "tools/browser_testing/browser_gate_suites",
        0,
        "any",
        &[normal("sqlx")],
    );
    workspace.layout_crate(
        "tools/staging/acknowledgement_dropping_relay",
        0,
        "any",
        &[normal("sqlx")],
    );
    let found = findings(&workspace);
    assert_eq!(found.len(), 2, "{found:#?}");
    for crate_path in [
        "tools/browser_testing/browser_gate_suites/Cargo.toml",
        "tools/staging/acknowledgement_dropping_relay/Cargo.toml",
    ] {
        assert!(
            found
                .iter()
                .any(|f| f.contains(crate_path) && f.contains("sqlx — sqlx and axum")),
            "{crate_path}: {found:#?}"
        );
    }
}

#[test]
fn crate_tiers_axum_from_a_harness_server_in_the_xtask_closure_is_rule_6() {
    let mut workspace = green_workspace("tiers-harness-xtask");
    workspace.layout_crate(
        "tools/browser_testing/browser_gate_suites",
        0,
        "any",
        &[normal("axum")],
    );
    workspace.layout_crate(
        "tools/commands/xtask",
        1,
        "any",
        &[normal("browser_gate_suites")],
    );
    let found = findings(&workspace);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].contains("tools/browser_testing/browser_gate_suites/Cargo.toml")
            && found[0].contains("axum enters the dependency closure of xtask"),
        "{found:#?}"
    );
}

#[test]
fn crate_tiers_browser_leptos_and_map_noun_firewalls_hold() {
    let mut workspace = green_workspace("tiers-browser");
    workspace.layout_crate(
        "crates/mission_editing/map_editing_tools",
        0,
        "wasm32",
        &[normal("web-sys")],
    );
    workspace.layout_crate("crates/terrain/road_network", 0, "any", &[normal("leptos")]);
    workspace.layout_crate("crates/graphics/render_primitives", 0, "any", &[]);
    workspace.write(
        "crates/graphics/render_primitives/src/batch.rs",
        "pub struct TerrainBatch;\n",
    );
    let found = findings(&workspace);
    assert_eq!(found.len(), 3, "{found:#?}");
    assert!(
        found
            .iter()
            .any(|f| f.contains("mission editing names no browser crate"))
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("leptos lives only in frontend crates"))
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("render_primitives/src/batch.rs:1"))
    );
}

#[test]
fn crate_tiers_a_wasm_only_crate_is_reached_only_from_a_wasm32_table() {
    let mut workspace = FixtureWorkspace::new("tiers-wasm");
    workspace.layout_crate("crates/foundation/browser_platform", 0, "wasm32", &[]);
    workspace.layout_crate(
        "crates/frontend/foundation/frontend_ui",
        1,
        "any",
        &[normal("browser_platform")],
    );
    assert_one_finding(
        &workspace,
        "wasm-only crates/foundation/browser_platform is reached from [dependencies]",
    );
    let mut workspace = FixtureWorkspace::new("tiers-wasm-green");
    workspace.layout_crate("crates/foundation/browser_platform", 0, "wasm32", &[]);
    let wasm_table = Dependency {
        package: "browser_platform",
        table: "target.'cfg(target_arch = \"wasm32\")'.dependencies",
    };
    workspace.layout_crate(
        "crates/frontend/foundation/frontend_ui",
        1,
        "any",
        &[wasm_table],
    );
    assert_eq!(findings(&workspace), Vec::<String>::new());
}

#[test]
fn crate_tiers_a_stray_manifest_is_rule_1_and_test_trees_are_not_swept() {
    let workspace = green_workspace("tiers-stray");
    workspace.write(
        "apps/orphan/Cargo.toml",
        &application_manifest("orphan", ""),
    );
    workspace.write(
        "apps/api/tests/fixtures/sample/Cargo.toml",
        &application_manifest("sample", ""),
    );
    workspace.write("crates/foundation/newtype_ids/target/debug/Cargo.toml", "");
    let found = findings(&workspace);
    assert_eq!(found, vec!["rule 1: apps/orphan/Cargo.toml is not a workspace member — add the folder to the root [workspace] members".to_string()]);
}

#[test]
fn crate_tiers_a_missing_declaration_wrong_category_or_name_is_rule_2_or_3() {
    let mut workspace = FixtureWorkspace::new("tiers-declaration");
    workspace.member("crates/foundation/bare", "[package]\nname = \"bare\"\n");
    workspace.member(
        "crates/foundation/misnamed",
        "[package]\nname = \"other_name\"\n\n[package.metadata.layout]\ncategory = \"crates/mission\"\ntier = \"one\"\ntargets = \"native\"\n",
    );
    let found = findings(&workspace);
    assert!(found.iter().any(|f| {
        f.starts_with("rule 2: crates/foundation/bare declares no [package.metadata.layout]")
    }));
    for needle in [
        "declares category `crates/mission` but sits in `crates/foundation`",
        "tier `one`, not a whole number",
        "targets `native`",
        "is package `other_name`",
    ] {
        assert!(
            found.iter().any(|f| f.contains(needle)),
            "{needle}: {found:#?}"
        );
    }
}

/// The mission crates' isolation: a `crates/mission` crate reaching a world or graphics crate is a
/// forbidden category edge, and one reaching the parked map engine breaks rule 7, so no mission
/// code links the world, streaming or rendering tiers.
#[test]
fn crate_tiers_a_mission_crate_reaching_world_or_graphics_is_rule_5() {
    let mut workspace = green_workspace("tiers-mission-isolation");
    workspace.layout_crate("crates/terrain/terrain_elevation", 0, "any", &[]);
    workspace.layout_crate("crates/graphics/render_primitives", 0, "any", &[]);
    workspace.member("legacy/map_engine", &application_manifest("map_engine", ""));
    workspace.layout_crate(
        "crates/mission/mission_compiler",
        3,
        "any",
        &[
            normal("mission_model"),
            normal("terrain_elevation"),
            normal("render_primitives"),
            normal("map_engine"),
        ],
    );
    assert_one_finding(
        &workspace,
        "crates/mission may not depend on crates/terrain/terrain_elevation",
    );
    assert_one_finding(
        &workspace,
        "crates/mission may not depend on crates/graphics/render_primitives",
    );
    let found = findings(&workspace);
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("rule 7: crates/mission/mission_compiler/Cargo.toml")),
        "{found:#?}"
    );
}

#[test]
fn crate_tiers_legacy_edges_and_dev_edges_onto_apps_are_rules_7_and_8() {
    let mut workspace = green_workspace("tiers-parked-member");
    workspace.member("legacy/frontend", &application_manifest("frontend", ""));
    workspace.layout_crate(
        "crates/frontend/pages/account_pages",
        0,
        "any",
        &[normal("frontend")],
    );
    workspace.layout_crate(
        "crates/foundation/deterministic_random",
        0,
        "any",
        &[Dependency {
            package: "api",
            table: "dev-dependencies",
        }],
    );
    let found = findings(&workspace);
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("rule 7: crates/frontend/pages/account_pages/Cargo.toml")),
        "{found:#?}"
    );
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("rule 8: crates/foundation/deterministic_random/Cargo.toml")),
        "{found:#?}"
    );
}

#[test]
fn crate_tiers_a_missing_member_folder_did_not_run() {
    let workspace = green_workspace("tiers-missing");
    std::fs::remove_dir_all(workspace.root().join("crates/mission/mission_model")).unwrap();
    let report = check_crate_tiers(workspace.root(), SWEEP);
    assert_eq!(report.exit_code, 2);
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("CRATE-TIERS: FAIL (did not run)")
    );
}

#[test]
fn crate_tiers_this_checkout_passes() {
    let report = check_crate_tiers(&this_repository(), &["apps", "crates", "tools", "legacy"]);
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
}

/// The mission editing arm of the category matrix: foundation crates built for every target,
/// mission, mission editing, geometry, the static world's data, line of sight and overlay crates
/// are allowed; ballistics, streaming, graphics and a wasm-only foundation crate are not, even
/// from a wasm32 target table.
#[test]
fn crate_tiers_a_mission_editing_crate_reaches_only_its_matrix_categories() {
    let allowed = [
        "crates/geometry/camera_math",
        "crates/world_formats/world_file_formats",
        "crates/terrain/terrain_elevation",
        "crates/world_objects/building_interiors",
        "crates/line_of_sight/terrain_line_of_sight",
        "crates/map_overlay/unit_symbology",
    ];
    let mut workspace = green_workspace("tiers-mission-editing-allowed");
    for path in allowed {
        workspace.layout_crate(path, 0, "any", &[]);
    }
    workspace.layout_crate(
        "crates/mission_editing/mission_editing_session",
        0,
        "any",
        &[],
    );
    let mut dependencies = vec![
        normal("time_source"),
        normal("mission_model"),
        normal("mission_editing_session"),
    ];
    dependencies.extend(allowed.map(|path| normal(path.rsplit_once('/').unwrap().1)));
    workspace.layout_crate(
        "crates/mission_editing/map_editing_tools",
        3,
        "any",
        &dependencies,
    );
    assert_eq!(findings(&workspace), Vec::<String>::new());

    let mut workspace = green_workspace("tiers-mission-editing-refused");
    workspace.layout_crate("crates/ballistics/ballistic_flight", 0, "any", &[]);
    workspace.layout_crate("crates/streaming/chunk_scheduler", 0, "any", &[]);
    workspace.layout_crate("crates/graphics/render_primitives", 0, "any", &[]);
    workspace.layout_crate("crates/foundation/browser_platform", 0, "wasm32", &[]);
    workspace.layout_crate(
        "crates/mission_editing/map_editing_tools",
        1,
        "any",
        &[
            normal("ballistic_flight"),
            normal("chunk_scheduler"),
            normal("render_primitives"),
            Dependency {
                package: "browser_platform",
                table: "target.'cfg(target_arch = \"wasm32\")'.dependencies",
            },
        ],
    );
    let found = findings(&workspace);
    for refused in [
        "crates/ballistics/ballistic_flight",
        "crates/streaming/chunk_scheduler",
        "crates/graphics/render_primitives",
        "crates/foundation/browser_platform",
    ] {
        assert!(
            found.iter().any(|f| f
                .starts_with("rule 5: crates/mission_editing/map_editing_tools/Cargo.toml:")
                && f.contains(&format!(
                    "crates/mission_editing may not depend on {refused}"
                ))),
            "{refused} must be refused: {found:#?}"
        );
    }
    assert_eq!(found.len(), 4, "{found:#?}");
}

/// The mission editing browser scan: a green tree passes, and the bare word `web_sys`, `leptos`
/// or `wasm_bindgen` in any `.rs` file under the category — source, test or comment — is one
/// finding per line, while a crate whose name only starts with one is not.
#[test]
fn crate_tiers_a_browser_token_in_a_mission_editing_source_is_rule_6() {
    let mut workspace = green_workspace("tiers-mission-editing-scan");
    workspace.layout_crate("crates/mission_editing/map_editing_tools", 0, "any", &[]);
    workspace.write(
        "crates/mission_editing/map_editing_tools/src/ruler.rs",
        "//! The ruler takes its clock from the host.\nuse web_sysfs::open;\n\
         use leptosaur::prelude::*;\npub fn step(now: &dyn Fn() -> f64) -> f64 { now() }\n",
    );
    assert_eq!(findings(&workspace), Vec::<String>::new());

    workspace.write(
        "crates/mission_editing/map_editing_tools/src/ruler.rs",
        "use web_sys::window;\npub fn n() -> f64 { wasm_bindgen::JsValue::TRUE.as_f64().unwrap() }\n",
    );
    workspace.write(
        "crates/mission_editing/map_editing_tools/tests/ruler_cases.rs",
        "// park the leptos signal here\n",
    );
    let found = findings(&workspace);
    assert_eq!(
        found,
        [
            "rule 6: crates/mission_editing/map_editing_tools/src/ruler.rs:1: mission editing names \
             no browser crate, prose included: use web_sys::window;",
            "rule 6: crates/mission_editing/map_editing_tools/src/ruler.rs:2: mission editing names \
             no browser crate, prose included: pub fn n() -> f64 { \
             wasm_bindgen::JsValue::TRUE.as_f64().unwrap() }",
            "rule 6: crates/mission_editing/map_editing_tools/tests/ruler_cases.rs:1: mission \
             editing names no browser crate, prose included: // park the leptos signal here",
        ]
    );
}

/// The mission editing browser scan's anti-vacuity case: a category folder holding no `.rs` file
/// is a finding, because "no source names a browser" and "there is no source" read alike; a
/// checkout with no mission editing layer at all has nothing to judge.
#[test]
fn crate_tiers_an_empty_mission_editing_root_is_not_a_clean_scan() {
    let workspace = green_workspace("tiers-mission-editing-empty");
    workspace.write("crates/mission_editing/README.md", "# Mission editing\n");
    assert_eq!(
        findings(&workspace),
        [
            "rule 6: walked 0 .rs file(s) under crates/mission_editing — the mission editing \
          browser scan refuses a vacuous pass"
        ]
    );
    let workspace = green_workspace("tiers-mission-editing-absent");
    assert_eq!(findings(&workspace), Vec::<String>::new());
}
