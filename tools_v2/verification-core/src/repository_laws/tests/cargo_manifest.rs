//! Tests for [`super`] — the manifest subset the dependency and feature laws read.

use super::*;

const MANIFEST: &str = r#"
[package]
name = "website-api" # the server

[dependencies]
axum = { version = "0.8", features = ["macros", "multipart"] }
# website-frontend = { path = "../frontend" } is a comment, not an edge
renderer = { package = "website-graphics-engine", path = "../graphics-engine" }
shared.workspace = true
"quoted-name" = "1"

[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { version = "0.3", features = [
    "Window",
    "Document",
] }

[dependencies.website-map-engine]
path = "../map-engine"
default-features = false
features = ["scenario"]

[features]
default = []
failpoints = []
extra = ["failpoints", "dep:serde", "axum/tokio"]

[dev-dependencies]
website-api = { path = ".", features = ["failpoints"] }
"#;

fn edge<'a>(manifest: &'a CargoManifest, package: &str) -> &'a DependencyEdge {
    manifest
        .dependencies
        .iter()
        .find(|edge| edge.package == package)
        .unwrap_or_else(|| panic!("no edge on {package}: {:#?}", manifest.dependencies))
}

#[test]
fn the_package_name_ignores_a_trailing_comment() {
    assert_eq!(
        parse_manifest(MANIFEST).package_name.as_deref(),
        Some("website-api")
    );
}

#[test]
fn every_dependency_table_and_spelling_yields_an_edge() {
    let manifest = parse_manifest(MANIFEST);
    let packages: Vec<&str> = manifest
        .dependencies
        .iter()
        .map(|edge| edge.package.as_str())
        .collect();
    assert_eq!(
        packages,
        [
            "axum",
            "website-graphics-engine",
            "shared",
            "quoted-name",
            "web-sys",
            "website-map-engine",
            "website-api",
        ]
    );
    let renamed = edge(&manifest, "website-graphics-engine");
    assert_eq!(renamed.key, "renderer");
    assert_eq!(renamed.path.as_deref(), Some("../graphics-engine"));
    assert_eq!(
        edge(&manifest, "web-sys").table,
        "target.'cfg(target_arch = \"wasm32\")'.dependencies"
    );
    assert_eq!(edge(&manifest, "web-sys").features, ["Window", "Document"]);
    assert_eq!(edge(&manifest, "axum").features, ["macros", "multipart"]);
    assert_eq!(edge(&manifest, "axum").line_no, 6);
}

#[test]
fn a_dependency_subtable_collects_its_fields() {
    let manifest = parse_manifest(MANIFEST);
    let map_engine = edge(&manifest, "website-map-engine");
    assert_eq!(map_engine.table, "dependencies");
    assert_eq!(map_engine.path.as_deref(), Some("../map-engine"));
    assert_eq!(map_engine.features, ["scenario"]);
}

#[test]
fn a_dev_dependency_edge_is_recognised_as_one() {
    let manifest = parse_manifest(MANIFEST);
    let self_edge = edge(&manifest, "website-api");
    assert!(self_edge.is_dev_dependency());
    assert_eq!(self_edge.features, ["failpoints"]);
    assert!(!edge(&manifest, "axum").is_dev_dependency());
}

#[test]
fn features_list_what_they_enable() {
    let manifest = parse_manifest(MANIFEST);
    assert_eq!(
        manifest.feature("default").unwrap().enables,
        Vec::<String>::new()
    );
    assert_eq!(
        manifest.feature("failpoints").unwrap().enables,
        Vec::<String>::new()
    );
    assert_eq!(
        manifest.feature("extra").unwrap().enables,
        ["failpoints", "dep:serde", "axum/tokio"]
    );
    assert!(manifest.feature("absent").is_none());
}

#[test]
fn a_missing_manifest_is_a_read_that_did_not_run() {
    let missing = std::path::Path::new("/nonexistent/tbd-laws/Cargo.toml");
    assert!(matches!(
        read_manifest(missing),
        Err(NotRun::TargetMissing(_))
    ));
}
