//! Tests for [`super`] — the dependency directions of the website crates and the test-only
//! feature rule.

use super::*;
use crate::cargo_manifest::parse_manifest;
use crate::temporary_checkout::TemporaryCheckout;

/// A checkout whose five website crates declare only the edges the layer order allows.
fn layered_checkout(name: &str) -> TemporaryCheckout {
    let checkout = TemporaryCheckout::empty(name);
    checkout.write(
        "legacy/graphics_engine/Cargo.toml",
        "[package]\nname = \"graphics_engine\"\n\n[dependencies]\nwgpu = \"29\"\n",
    );
    checkout.write(
        "legacy/map_engine/Cargo.toml",
        "[package]\nname = \"map_engine\"\n\n[dependencies]\n\
         graphics_engine = { path = \"../graphics_engine\", optional = true }\n",
    );
    checkout.write(
        "apps/frontend/Cargo.toml",
        "[package]\nname = \"frontend\"\n\n[dependencies]\n\
         map_engine = { path = \"../../legacy/map_engine\" }\n\
         # graphics_engine is reached through the map engine\n",
    );
    checkout.write(
        "apps/api/Cargo.toml",
        "[package]\nname = \"api\"\n\n[dependencies]\n\
         map_engine = { path = \"../../legacy/map_engine\" }\n",
    );
    checkout.write(
        "apps/offline_service_worker/Cargo.toml",
        "[package]\nname = \"offline_service_worker\"\n\n[dependencies]\n\
         serde = \"1\"\n",
    );
    checkout
}

#[test]
fn the_layer_order_holds_with_no_finding() {
    let checkout = layered_checkout("directions-clean");
    assert!(
        crate_dependency_findings(checkout.root())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_forbidden_edge_in_any_table_or_spelling_is_a_finding() {
    for (crate_rel, package_name, edge, forbidden) in [
        (
            "legacy/graphics_engine",
            "graphics_engine",
            "[dependencies]\nm = { package = \"map_engine\", path = \"../map_engine\" }",
            "map_engine",
        ),
        (
            "legacy/map_engine",
            "map_engine",
            "[dev-dependencies]\napi = { path = \"../../apps/api\" }",
            "api",
        ),
        (
            "apps/frontend",
            "frontend",
            "[target.'cfg(target_arch = \"wasm32\")'.dependencies]\ngraphics_engine = { path = \"../../legacy/graphics_engine\" }",
            "graphics_engine",
        ),
        (
            "apps/api",
            "api",
            "[dependencies.frontend]\npath = \"../../apps/frontend\"",
            "frontend",
        ),
        (
            "apps/api",
            "api",
            "[build-dependencies]\ngraphics_engine = { path = \"../../legacy/graphics_engine\" }",
            "graphics_engine",
        ),
    ] {
        let checkout = layered_checkout("directions-breach");
        checkout.write(
            &format!("{crate_rel}/Cargo.toml"),
            &format!("[package]\nname = \"{package_name}\"\n\n{edge}\n"),
        );
        let findings = crate_dependency_findings(checkout.root()).unwrap();
        assert_eq!(findings.len(), 1, "{edge}: {findings:#?}");
        assert!(findings[0].manifest.starts_with(crate_rel));
        assert_eq!(findings[0].package, forbidden, "{edge}");
        assert!(
            findings[0]
                .rendered()
                .contains(&format!(" depends on {forbidden} ")),
            "{findings:#?}"
        );
    }
}

#[test]
fn the_offline_service_worker_may_link_none_of_the_server_page_or_renderer() {
    for (forbidden, path) in [
        ("api", "../api"),
        ("frontend", "../frontend"),
        ("graphics_engine", "../../legacy/graphics_engine"),
    ] {
        let checkout = layered_checkout("directions-offline-worker");
        checkout.write(
            "apps/offline_service_worker/Cargo.toml",
            &format!(
                "[package]\nname = \"offline_service_worker\"\n\n\
                 [target.'cfg(target_arch = \"wasm32\")'.dependencies]\n\
                 {forbidden} = {{ path = \"{path}\" }}\n"
            ),
        );
        let findings = crate_dependency_findings(checkout.root()).unwrap();
        assert_eq!(findings.len(), 1, "{forbidden}: {findings:#?}");
        assert_eq!(
            findings[0].manifest,
            "apps/offline_service_worker/Cargo.toml"
        );
        assert_eq!(findings[0].package, forbidden);
        assert_eq!(findings[0].reason, OFFLINE_SERVICE_WORKER_RULE.reason);
    }
    assert!(CRATE_DEPENDENCY_RULES.contains(&OFFLINE_SERVICE_WORKER_RULE));
}

#[test]
fn a_missing_manifest_is_a_check_that_did_not_run() {
    let checkout = layered_checkout("directions-missing");
    std::fs::remove_file(checkout.root().join("apps/frontend/Cargo.toml")).unwrap();
    assert!(matches!(
        crate_dependency_findings(checkout.root()),
        Err(NotRun::TargetMissing(_))
    ));
}

const TEST_ONLY: &str = "\
[package]
name = \"api\"

[features]
failpoints = []

[dev-dependencies]
api = { path = \".\", features = [\"failpoints\"] }
";

#[test]
fn a_feature_enabled_only_by_the_self_dev_dependency_is_test_only() {
    assert!(test_only_feature_findings(&parse_manifest(TEST_ONLY), "failpoints").is_empty());
}

#[test]
fn every_path_out_of_the_test_build_is_a_finding() {
    for (manifest, expected) in [
        (
            TEST_ONLY.replace(
                "failpoints = []",
                "default = [\"failpoints\"]\nfailpoints = []",
            ),
            "feature `default` enables",
        ),
        (
            TEST_ONLY.replace(
                "failpoints = []",
                "failpoints = []\nchaos = [\"api/failpoints\"]",
            ),
            "feature `chaos` enables",
        ),
        (
            format!(
                "{TEST_ONLY}\n[dependencies]\napi = {{ path = \".\", features = [\"failpoints\"] }}\n"
            ),
            "[dependencies] api enables",
        ),
        (
            TEST_ONLY.replace("api = { path", "other = { package = \"other\", path"),
            "no dev-dependency of `api` on itself",
        ),
        (
            TEST_ONLY.replace("failpoints = []\n", ""),
            "[features] does not declare `failpoints`",
        ),
    ] {
        let findings = test_only_feature_findings(&parse_manifest(&manifest), "failpoints");
        assert!(
            findings.iter().any(|finding| finding.contains(expected)),
            "{expected}: {findings:#?}"
        );
    }
}
