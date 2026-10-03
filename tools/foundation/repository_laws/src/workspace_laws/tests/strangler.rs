//! Tests for [`super`] — edges onto members under `legacy/` and re-export shims.

use super::*;
use crate::temporary_checkout::this_repository;
use crate::workspace_laws::fixture_workspace::{FixtureWorkspace, application_manifest, normal};

fn workspace(name: &str) -> FixtureWorkspace {
    let mut workspace = FixtureWorkspace::new(name);
    workspace.layout_crate("crates/mission/mission_model", 0, "any", &[]);
    workspace.member("legacy/map_engine", &application_manifest("map_engine", ""));
    workspace.write("legacy/map_engine/src/lib.rs", "pub mod data;\n");
    workspace.member(
        "apps/frontend",
        &application_manifest(
            "frontend",
            "map_engine = { path = \"../../legacy/map_engine\" }\n",
        ),
    );
    workspace
}

#[test]
fn strangler_apps_may_depend_on_legacy_while_it_exists() {
    let workspace = workspace("strangler-green");
    let report = check_strangler(workspace.root());
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert!(report.lines[0].contains("3 workspace member(s), 1 under legacy/"));
}

#[test]
fn strangler_a_new_crate_depending_on_legacy_is_a_finding() {
    let mut workspace = workspace("strangler-edge");
    workspace.layout_crate(
        "crates/mission/mission_payload",
        0,
        "any",
        &[normal("map_engine")],
    );
    let found = strangler_outcome(workspace.root()).unwrap().findings;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].starts_with("crates/mission/mission_payload/Cargo.toml:"));
    assert!(found[0].contains("depends on legacy/map_engine"));
}

#[test]
fn strangler_a_reexport_of_a_new_crate_inside_legacy_is_a_shim() {
    let workspace = workspace("strangler-shim");
    workspace.write(
        "legacy/map_engine/src/data.rs",
        "pub use mission_model::Mission;\npub(crate) use mission_model::Unit;\npub use std::fmt;\n",
    );
    let report = check_strangler(workspace.root());
    assert_eq!(report.exit_code, 1, "{}", report.lines.join("\n"));
    assert!(report.lines.iter().any(|line| line == "FAIL: legacy/map_engine/src/data.rs:1: shim — a member under legacy/ re-exports `mission_model`; move the callers instead"));
    assert_eq!(
        report
            .lines
            .iter()
            .filter(|line| line.starts_with("FAIL:"))
            .count(),
        1
    );
}

#[test]
fn strangler_reads_only_pub_use_lines_as_reexports() {
    assert_eq!(
        reexported_crate("pub use mission_model::Mission;"),
        Some("mission_model")
    );
    assert_eq!(
        reexported_crate("    pub use ::mission_model::Mission;"),
        Some("mission_model")
    );
    assert_eq!(reexported_crate("pub use crate::data::Mission;"), None);
    assert_eq!(
        reexported_crate("pub(crate) use mission_model::Mission;"),
        None
    );
    assert_eq!(reexported_crate("use mission_model::Mission;"), None);
}

#[test]
fn strangler_reads_bare_aliased_and_grouped_crate_reexports() {
    let at = |text: &str| reexported_crates(text);
    let one = |name: &str| vec![(1, name.to_owned())];
    assert_eq!(at("pub use mission_model;"), one("mission_model"));
    assert_eq!(
        at("#[cfg(test)]\npub use satellite_imagery as streamer;"),
        vec![(2, "satellite_imagery".to_owned())]
    );
    assert_eq!(
        at("    pub use ::mission_model as model;"),
        one("mission_model")
    );
    assert_eq!(at("pub use mission_model::Mission;"), one("mission_model"));
    assert_eq!(
        at("pub use {mission_model::{Mission, Unit}, std::fmt, crate::data};"),
        vec![(1, "mission_model".to_owned()), (1, "std".to_owned())]
    );
    assert_eq!(
        at("pub use mission_model as model; // a shim\n"),
        one("mission_model")
    );
    assert_eq!(at("pub use crate::data as model;"), Vec::new());
    assert_eq!(at("pub use self::data;\npub use super::data;"), Vec::new());
    assert_eq!(at("pub(crate) use mission_model as model;"), Vec::new());
    assert_eq!(at("use mission_model as model;"), Vec::new());
    assert_eq!(at("pub user_name: String,"), Vec::new());
}

#[test]
fn strangler_reads_a_reexport_spread_over_several_lines() {
    let text = "/// Docs.\npub use\n    mission_model\n    as model;\n\npub use {\n    // the old path\n    std::fmt,\n    mission_model::{\n        Mission,\n    },\n};\n";
    assert_eq!(
        reexported_crates(text),
        vec![
            (2, "mission_model".to_owned()),
            (6, "std".to_owned()),
            (6, "mission_model".to_owned()),
        ]
    );
}

#[test]
fn strangler_an_aliased_or_multi_line_crate_reexport_inside_legacy_is_a_shim() {
    let workspace = workspace("strangler-alias");
    workspace.write(
        "legacy/map_engine/src/data.rs",
        "pub use mission_model as model;\npub use mission_model;\npub use\n    mission_model::Unit;\npub use std as standard;\n",
    );
    let report = check_strangler(workspace.root());
    assert_eq!(report.exit_code, 1, "{}", report.lines.join("\n"));
    let failing: Vec<&String> = report
        .lines
        .iter()
        .filter(|line| line.starts_with("FAIL:"))
        .collect();
    let expected: Vec<String> = [1, 2, 3]
        .iter()
        .map(|line| {
            format!(
                "FAIL: legacy/map_engine/src/data.rs:{line}: shim — a member under legacy/ \
                 re-exports `mission_model`; move the callers instead"
            )
        })
        .collect();
    assert_eq!(
        failing,
        expected.iter().collect::<Vec<_>>(),
        "{}",
        report.lines.join("\n")
    );
}

#[test]
fn strangler_this_checkout_passes() {
    let report = check_strangler(&this_repository());
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
}
