//! The test-only feature rule: a feature only test builds compile in.
//!
//! **Role:** answers whether a test-only feature (such as `failpoints`) can reach a build that is
//! not a test build: it must be declared, off by default, enabled by no other feature and by no
//! dependency edge but the crate's own dev-dependency on itself, and that self edge must exist.
//! **Position:** reads one manifest parsed by [`super::cargo_manifest`]; consumed by the
//! `engineering_laws` test binary of `api_server`, which holds the failpoints crate to it.
//! **Signals & state:** none; pure functions over a parsed manifest.
//! **Invariants:** every dependency table counts — normal, dev, build and target-specific — and
//! an edge enabling the feature counts under its real package name, so no spelling of an enabling
//! edge slips past; a feature enabled as `<package>/<feature>` counts as enabling it.

use super::cargo_manifest::CargoManifest;

/// Every way `feature` of `manifest` could reach a build that is not a test build.
///
/// The feature must be declared; no feature — `default` included — may enable it; no
/// dependency edge may enable it except a dev-dependency on the manifest's own package; and at
/// least one such self edge must exist, or the tests never compile it in.
pub fn test_only_feature_findings(manifest: &CargoManifest, feature: &str) -> Vec<String> {
    let mut findings = Vec::new();
    let package = manifest.package_name.as_deref().unwrap_or_default();
    if manifest.feature(feature).is_none() {
        findings.push(format!("[features] does not declare `{feature}`"));
    }
    for declared in &manifest.features {
        let enables_it = declared.enables.iter().any(|item| {
            item == feature
                || item
                    .rsplit_once('/')
                    .is_some_and(|(_, tail)| tail == feature)
        });
        if enables_it && declared.name != feature {
            findings.push(format!(
                "line {}: feature `{}` enables `{feature}`, so a build that is not a test build \
                 carries it",
                declared.line_no, declared.name
            ));
        }
    }
    let mut self_edges = 0;
    for edge in &manifest.dependencies {
        if !edge.features.iter().any(|enabled| enabled == feature) {
            continue;
        }
        if edge.is_dev_dependency() && edge.package == package {
            self_edges += 1;
        } else {
            findings.push(format!(
                "line {}: [{}] {} enables `{feature}`; only the crate's dev-dependency on itself \
                 may",
                edge.line_no, edge.table, edge.package
            ));
        }
    }
    if self_edges == 0 {
        findings.push(format!(
            "no dev-dependency of `{package}` on itself enables `{feature}`, so no test build \
             compiles it in"
        ));
    }
    findings
}

#[cfg(test)]
#[path = "tests/test_only_features.rs"]
mod tests;
