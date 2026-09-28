//! The dependency-direction law between the website crates, and the test-only feature rule.
//!
//! **Role:** checks that no website crate declares a dependency edge against the layer order of
//! `CLAUDE.md` law 6 — the renderer knows no map, page or server; the map engine knows no page or
//! server; the frontend reaches the renderer only through the map engine and never links the
//! server; the server links the map engine's mission domain and neither the renderer nor the
//! frontend; the offline service worker links none of the server, the frontend or the renderer —
//! and that a test-only feature is declared, off by default and enabled only by the
//! crate's own dev-dependency on itself.
//! **Position:** reads manifests through [`super::cargo_manifest`]; consumed by the
//! `engineering_laws` test binary of `website-api`.
//! **Signals & state:** none; pure functions over the checkout.
//! **Invariants:** every dependency table counts — normal, dev, build and target-specific — and
//! a renamed dependency counts under its real package name, so no spelling of an edge slips past.
//! A missing manifest is [`NotRun::TargetMissing`], never a crate with no edges.

use std::path::Path;

use super::cargo_manifest::{CargoManifest, read_manifest};
use crate::verdict::NotRun;

/// One crate's forbidden dependency edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrateDependencyRule {
    /// The crate's folder, relative to the repository root.
    pub crate_rel: &'static str,
    /// Packages the crate may not depend on, directly, in any table.
    pub forbidden_packages: &'static [&'static str],
    /// Why the edge would break the layer order.
    pub reason: &'static str,
}

/// The renderer: pure GPU primitives, no map concept, no page, no server.
pub const GRAPHICS_ENGINE_RULE: CrateDependencyRule = CrateDependencyRule {
    crate_rel: "apps/website/graphics-engine",
    forbidden_packages: &["website-map-engine", "website-frontend", "website-api"],
    reason: "the renderer knows no map concept, no page and no server; the arrow runs \
             map-engine -> graphics-engine only",
};

/// The map engine: map graphics and the mission domain, no page and no server.
pub const MAP_ENGINE_RULE: CrateDependencyRule = CrateDependencyRule {
    crate_rel: "apps/website/map-engine",
    forbidden_packages: &["website-frontend", "website-api"],
    reason: "the map engine serves the frontend and the server and depends on neither",
};

/// The frontend: presentation over the map engine, never the renderer or the server directly.
pub const FRONTEND_RULE: CrateDependencyRule = CrateDependencyRule {
    crate_rel: "apps/website/frontend",
    forbidden_packages: &["website-graphics-engine", "website-api"],
    reason: "the frontend reaches the renderer through website-map-engine and the server over \
             HTTP, never by linking either",
};

/// The server: the map engine's mission domain only, never the renderer or the frontend.
pub const WEBSITE_API_RULE: CrateDependencyRule = CrateDependencyRule {
    crate_rel: "apps/website/api_v2",
    forbidden_packages: &["website-graphics-engine", "website-frontend"],
    reason: "the server links website-map-engine for the mission domain alone and links neither \
             the renderer nor the frontend",
};

/// The offline service worker: cache policy the page also links, never the server, the page or
/// the renderer.
pub const OFFLINE_SERVICE_WORKER_RULE: CrateDependencyRule = CrateDependencyRule {
    crate_rel: "apps/website/offline-service-worker",
    forbidden_packages: &["website-api", "website-frontend", "website-graphics-engine"],
    reason: "the offline service worker is a leaf the frontend links; it reaches the server over \
             HTTP and links neither the page nor the renderer",
};

/// Every dependency-direction rule of the website crates.
pub const CRATE_DEPENDENCY_RULES: &[CrateDependencyRule] = &[
    GRAPHICS_ENGINE_RULE,
    MAP_ENGINE_RULE,
    FRONTEND_RULE,
    WEBSITE_API_RULE,
    OFFLINE_SERVICE_WORKER_RULE,
];

/// One forbidden dependency edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyFinding {
    /// The manifest, repository-relative.
    pub manifest: String,
    /// The table the edge sits in.
    pub table: String,
    /// The forbidden package.
    pub package: String,
    /// 1-based line of the edge.
    pub line_no: usize,
    /// Why the edge is forbidden.
    pub reason: &'static str,
}

impl DependencyFinding {
    /// `manifest:line: [table] depends on package — reason`.
    pub fn rendered(&self) -> String {
        format!(
            "{}:{}: [{}] depends on {} — {}",
            self.manifest, self.line_no, self.table, self.package, self.reason
        )
    }
}

/// Every forbidden edge of `rule` in its crate's manifest under `repo_root`.
pub fn rule_findings(
    repo_root: &Path,
    rule: &CrateDependencyRule,
) -> Result<Vec<DependencyFinding>, NotRun> {
    let manifest_rel = format!("{}/Cargo.toml", rule.crate_rel);
    let manifest = read_manifest(&repo_root.join(&manifest_rel))?;
    Ok(manifest
        .dependencies
        .iter()
        .filter(|edge| rule.forbidden_packages.contains(&edge.package.as_str()))
        .map(|edge| DependencyFinding {
            manifest: manifest_rel.clone(),
            table: edge.table.clone(),
            package: edge.package.clone(),
            line_no: edge.line_no,
            reason: rule.reason,
        })
        .collect())
}

/// Every forbidden edge of every rule in [`CRATE_DEPENDENCY_RULES`].
pub fn crate_dependency_findings(repo_root: &Path) -> Result<Vec<DependencyFinding>, NotRun> {
    let mut findings = Vec::new();
    for rule in CRATE_DEPENDENCY_RULES {
        findings.extend(rule_findings(repo_root, rule)?);
    }
    Ok(findings)
}

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
#[path = "tests/crate_dependencies.rs"]
mod tests;
