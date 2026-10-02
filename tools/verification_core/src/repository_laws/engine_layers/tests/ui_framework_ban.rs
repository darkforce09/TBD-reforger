//! Tests for [`super`] — the map engine's whole-crate UI-framework ban.

use super::super::fixture_repository::Repo;
use super::*;

/// A clean map-engine manifest: browser bindings, the renderer and the CRDT crate, no framework.
const CLEAN_MAP_MANIFEST: &str = "\
[package]
name = \"website-map-engine\"

[dependencies]
website-graphics-engine = { path = \"../graphics-engine\", optional = true }
yrs = \"0.25\"
# leptos = \"0.8\" is prose in a comment, not an edge

[target.'cfg(target_arch = \"wasm32\")'.dependencies]
wasm-bindgen = \"0.2\"
web-sys = { version = \"0.3\", features = [
    \"Window\",
    \"Document\",
] }
";

fn map_manifest(r: &Repo, body: &str) {
    std::fs::write(r.0.join("apps/website/map-engine/Cargo.toml"), body).unwrap();
}

#[test]
fn a_clean_map_engine_names_no_ui_framework() {
    let r = Repo::new("ui-ban-clean");
    map_manifest(&r, CLEAN_MAP_MANIFEST);
    r.map(
        "world/terrain/satellite/basemap.rs",
        "/// Sharp on `localhost` too (`cargo xtask mk leptos`).\npub fn load() {}\n",
    );
    let scan = map_engine_ui_framework_findings(&r.0).unwrap();
    assert!(scan.findings.is_empty(), "{:#?}", scan.findings);
    assert!(scan.source_files > 0);
}

#[test]
fn a_ui_framework_edge_in_any_table_fails() {
    for (table, edge) in [
        ("[dependencies]", "leptos = \"0.8\""),
        ("[dev-dependencies]", "leptos_router = \"0.8\""),
        (
            "[target.'cfg(target_arch = \"wasm32\")'.dependencies]",
            "dioxus-web = \"0.6\"",
        ),
        (
            "[dependencies]",
            "ui = { package = \"yew\", version = \"0.21\" }",
        ),
    ] {
        let r = Repo::new("ui-ban-edge");
        map_manifest(&r, &format!("{CLEAN_MAP_MANIFEST}\n{table}\n{edge}\n"));
        let scan = map_engine_ui_framework_findings(&r.0).unwrap();
        assert_eq!(scan.findings.len(), 1, "{edge}: {:#?}", scan.findings);
        assert!(
            scan.findings[0].contains("depends on the UI framework"),
            "{:#?}",
            scan.findings
        );
    }
}

#[test]
fn a_ui_framework_import_anywhere_in_the_crate_fails() {
    let r = Repo::new("ui-ban-import");
    map_manifest(&r, CLEAN_MAP_MANIFEST);
    r.map("frame/panel.rs", "use leptos::prelude::RwSignal;\n");
    r.map("streaming/host.rs", "extern crate yew;\n");
    let scan = map_engine_ui_framework_findings(&r.0).unwrap();
    assert_eq!(scan.findings.len(), 2, "{:#?}", scan.findings);
    assert!(scan.findings[0].starts_with("apps/website/map-engine/src/frame/panel.rs:1:"));
    assert!(scan.findings[1].starts_with("apps/website/map-engine/src/streaming/host.rs:1:"));
}

#[test]
fn a_missing_manifest_or_source_tree_does_not_run() {
    let r = Repo::new("ui-ban-missing");
    assert!(matches!(
        map_engine_ui_framework_findings(&r.0),
        Err(NotRun::TargetMissing(_))
    ));
    map_manifest(&r, CLEAN_MAP_MANIFEST);
    std::fs::remove_dir_all(r.0.join("apps/website/map-engine/src")).unwrap();
    assert!(matches!(
        map_engine_ui_framework_findings(&r.0),
        Err(NotRun::TargetMissing(_))
    ));
}
