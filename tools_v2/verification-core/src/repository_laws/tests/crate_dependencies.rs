//! Tests for [`super`] — the dependency directions of the website crates and the test-only
//! feature rule.

use super::*;
use crate::repository_laws::cargo_manifest::parse_manifest;
use crate::repository_laws::temporary_checkout::TemporaryCheckout;

/// A checkout whose five website crates declare only the edges the layer order allows.
fn layered_checkout(name: &str) -> TemporaryCheckout {
    let checkout = TemporaryCheckout::empty(name);
    checkout.write(
        "apps/website/graphics-engine/Cargo.toml",
        "[package]\nname = \"website-graphics-engine\"\n\n[dependencies]\nwgpu = \"29\"\n",
    );
    checkout.write(
        "apps/website/map-engine/Cargo.toml",
        "[package]\nname = \"website-map-engine\"\n\n[dependencies]\n\
         website-graphics-engine = { path = \"../graphics-engine\", optional = true }\n",
    );
    checkout.write(
        "apps/website/frontend/Cargo.toml",
        "[package]\nname = \"website-frontend\"\n\n[dependencies]\n\
         website-map-engine = { path = \"../map-engine\" }\n\
         # website-graphics-engine is reached through the map engine\n",
    );
    checkout.write(
        "apps/website/api_v2/Cargo.toml",
        "[package]\nname = \"website-api\"\n\n[dependencies]\n\
         website-map-engine = { path = \"../map-engine\" }\n",
    );
    checkout.write(
        "apps/website/offline-service-worker/Cargo.toml",
        "[package]\nname = \"website-offline-service-worker\"\n\n[dependencies]\n\
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
    for (crate_rel, package_name, edge) in [
        (
            "apps/website/graphics-engine",
            "website-graphics-engine",
            "[dependencies]\nm = { package = \"website-map-engine\", path = \"../map-engine\" }",
        ),
        (
            "apps/website/map-engine",
            "website-map-engine",
            "[dev-dependencies]\nwebsite-api = { path = \"../api_v2\" }",
        ),
        (
            "apps/website/frontend",
            "website-frontend",
            "[target.'cfg(target_arch = \"wasm32\")'.dependencies]\nwebsite-graphics-engine = { path = \"../graphics-engine\" }",
        ),
        (
            "apps/website/api_v2",
            "website-api",
            "[dependencies.website-frontend]\npath = \"../frontend\"",
        ),
        (
            "apps/website/api_v2",
            "website-api",
            "[build-dependencies]\nwebsite-graphics-engine = { path = \"../graphics-engine\" }",
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
        assert!(
            findings[0].rendered().contains(" depends on website-"),
            "{findings:#?}"
        );
    }
}

#[test]
fn the_offline_service_worker_may_link_none_of_the_server_page_or_renderer() {
    for (forbidden, folder) in [
        ("website-api", "api_v2"),
        ("website-frontend", "frontend"),
        ("website-graphics-engine", "graphics-engine"),
    ] {
        let checkout = layered_checkout("directions-offline-worker");
        checkout.write(
            "apps/website/offline-service-worker/Cargo.toml",
            &format!(
                "[package]\nname = \"website-offline-service-worker\"\n\n\
                 [target.'cfg(target_arch = \"wasm32\")'.dependencies]\n\
                 {forbidden} = {{ path = \"../{folder}\" }}\n"
            ),
        );
        let findings = crate_dependency_findings(checkout.root()).unwrap();
        assert_eq!(findings.len(), 1, "{forbidden}: {findings:#?}");
        assert_eq!(
            findings[0].manifest,
            "apps/website/offline-service-worker/Cargo.toml"
        );
        assert_eq!(findings[0].package, forbidden);
        assert_eq!(findings[0].reason, OFFLINE_SERVICE_WORKER_RULE.reason);
    }
    assert!(CRATE_DEPENDENCY_RULES.contains(&OFFLINE_SERVICE_WORKER_RULE));
}

#[test]
fn a_missing_manifest_is_a_check_that_did_not_run() {
    let checkout = layered_checkout("directions-missing");
    std::fs::remove_file(checkout.root().join("apps/website/frontend/Cargo.toml")).unwrap();
    assert!(matches!(
        crate_dependency_findings(checkout.root()),
        Err(NotRun::TargetMissing(_))
    ));
}

const TEST_ONLY: &str = "\
[package]
name = \"website-api\"

[features]
failpoints = []

[dev-dependencies]
website-api = { path = \".\", features = [\"failpoints\"] }
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
                "failpoints = []\nchaos = [\"website-api/failpoints\"]",
            ),
            "feature `chaos` enables",
        ),
        (
            format!(
                "{TEST_ONLY}\n[dependencies]\nwebsite-api = {{ path = \".\", features = [\"failpoints\"] }}\n"
            ),
            "[dependencies] website-api enables",
        ),
        (
            TEST_ONLY.replace(
                "website-api = { path",
                "other = { package = \"other\", path",
            ),
            "no dev-dependency of `website-api` on itself",
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
