//! Tests for [`super`] — a feature enabled only by the crate's dev-dependency on itself is
//! test-only, and every other path into a build is a finding.

use super::*;
use crate::cargo_manifest::parse_manifest;

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
