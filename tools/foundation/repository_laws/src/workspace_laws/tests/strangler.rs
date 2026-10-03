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

/// No tool is exempt: a tool binary at `tools/<name>`, which carries no layout table, and a tool
/// crate both report their edge onto a member under `legacy/`.
#[test]
fn strangler_a_tool_depending_on_legacy_is_a_finding() {
    let mut workspace = workspace("strangler-tool-edge");
    workspace.member(
        "tools/xtask",
        &application_manifest(
            "xtask",
            "map_engine = { path = \"../../legacy/map_engine\" }\n",
        ),
    );
    workspace.layout_crate(
        "tools/commands/ci_task_catalog",
        0,
        "any",
        &[normal("map_engine")],
    );
    let found = strangler_outcome(workspace.root()).unwrap().findings;
    assert_eq!(found.len(), 2, "{found:#?}");
    for tool in [
        "tools/xtask/Cargo.toml:",
        "tools/commands/ci_task_catalog/Cargo.toml:",
    ] {
        assert!(
            found
                .iter()
                .any(|f| f.starts_with(tool) && f.contains("depends on legacy/map_engine")),
            "{tool}: {found:#?}"
        );
    }
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
fn strangler_aliased_grouped_rooted_and_multi_line_reexports_inside_legacy_are_shims() {
    let workspace = workspace("strangler-alias-shim");
    workspace.write(
        "legacy/map_engine/src/data.rs",
        "pub use mission_model as model;\n\
         pub use ::mission_model as rooted;\n\
         pub use mission_model::{self as grouped};\n\
         pub use {std::fmt, mission_model as braced};\n\
         pub use mission_model;\n\
         pub extern crate mission_model as external;\n\
         pub use {\n    std::io,\n    mission_model as spread,\n};\n\
         pub use\n    mission_model::Unit;\n\
         pub use std as standard;\n\
         pub(crate) use mission_model as private;\n\
         use mission_model as local;\n\
         // pub use mission_model as commented;\n",
    );
    let report = check_strangler(workspace.root());
    assert_eq!(report.exit_code, 1, "{}", report.lines.join("\n"));
    let shim_lines: Vec<&String> = report
        .lines
        .iter()
        .filter(|line| line.starts_with("FAIL:"))
        .collect();
    let expected: Vec<String> = [1, 2, 3, 4, 5, 6, 7, 11]
        .iter()
        .map(|line| {
            format!(
                "FAIL: legacy/map_engine/src/data.rs:{line}: shim — a member under legacy/ \
                 re-exports `mission_model`; move the callers instead"
            )
        })
        .collect();
    assert_eq!(
        shim_lines,
        expected.iter().collect::<Vec<_>>(),
        "{}",
        report.lines.join("\n")
    );
}

#[test]
fn strangler_reads_every_reexport_form_of_a_crate() {
    let one = |name: &str| vec![(1, name.to_owned())];
    let names = |names: &[&str]| -> Vec<(usize, String)> {
        names.iter().map(|name| (1, (*name).to_owned())).collect()
    };
    for (text, expected) in [
        ("pub use mission_model;", one("mission_model")),
        ("pub use mission_model::Mission;", one("mission_model")),
        ("pub use mission_model as model;", one("mission_model")),
        ("pub use ::mission_model as rooted;", one("mission_model")),
        (
            "    pub use ::mission_model as model;",
            one("mission_model"),
        ),
        (
            "pub use mission_model::{self as grouped};",
            one("mission_model"),
        ),
        (
            "pub use {mission_model as braced, ::other_crate::Item};",
            names(&["mission_model", "other_crate"]),
        ),
        (
            "pub use { std::{fmt, io}, mission_model::* };",
            names(&["std", "mission_model"]),
        ),
        (
            "pub use {mission_model::{Mission, Unit}, std::fmt, crate::data};",
            names(&["mission_model", "std"]),
        ),
        (
            "pub use {{mission_model, ::other_crate}, self::local};",
            names(&["mission_model", "other_crate"]),
        ),
        (
            "pub extern crate mission_model as external;",
            one("mission_model"),
        ),
        (
            "pub use mission_model as model; // a shim\n",
            one("mission_model"),
        ),
        (
            "#[cfg(test)]\npub use satellite_imagery as streamer;",
            vec![(2, "satellite_imagery".to_owned())],
        ),
        ("pub use {crate::data, self::local, super::parent};", vec![]),
        ("pub use crate::data as model;", vec![]),
        ("pub use self::data;\npub use super::data;", vec![]),
        ("pub(crate) use mission_model as private;", vec![]),
        ("pub(super) use mission_model::Mission;", vec![]),
        ("use mission_model as local;", vec![]),
        ("pub user_model: Model,", vec![]),
    ] {
        assert_eq!(reexported_crates(text), expected, "{text}");
    }
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
fn strangler_an_absent_legacy_folder_is_named_in_a_note() {
    let mut workspace = FixtureWorkspace::new("strangler-absent-parking-folder");
    workspace.layout_crate("crates/mission/mission_model", 0, "any", &[]);
    let report = check_strangler(workspace.root());
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert!(
        report
            .lines
            .iter()
            .any(|line| line.starts_with("note: legacy/ is absent")),
        "{}",
        report.lines.join("\n")
    );
    let parked = check_strangler(workspace_with_legacy().root());
    assert!(!parked.lines.iter().any(|line| line.contains("is absent")));
}

/// The green fixture of [`strangler_apps_may_depend_on_legacy_while_it_exists`].
fn workspace_with_legacy() -> FixtureWorkspace {
    workspace("strangler-parked-member")
}

#[test]
fn strangler_this_checkout_passes() {
    let report = check_strangler(&this_repository());
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
}
