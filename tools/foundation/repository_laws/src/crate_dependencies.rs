//! The dependency-direction law between the website applications, and the test-only feature rule.
//!
//! **Role:** checks that no website application declares a dependency edge against the layer
//! order of `CLAUDE.md` law 6 — the frontend never links the server; the server links the mission
//! and ballistics crates and neither a GPU crate nor the frontend; the offline service worker links
//! none of the server, the frontend or a GPU crate — and that a test-only feature is declared, off
//! by default and enabled only by the crate's own dev-dependency on itself. The library crates have
//! no row here: the crate-tier law judges every edge of theirs (its category matrix and its
//! firewalls), while the applications under `apps/` carry no layout declaration it could judge.
//! **Position:** reads manifests through [`super::cargo_manifest`]; consumed by the
//! `engineering_laws` test binary of `api`.
//! **Signals & state:** none; pure functions over the checkout.
//! **Invariants:** every dependency table counts — normal, dev, build and target-specific — and
//! a renamed dependency counts under its real package name, so no spelling of an edge slips past.
//! A missing manifest is [`NotRun::TargetMissing`], never a crate with no edges.

use std::path::Path;

use super::cargo_manifest::{CargoManifest, read_manifest};
use verification_core::verdict::NotRun;

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

/// The frontend: presentation over the map and paper-doll renderers, never the server directly.
pub const FRONTEND_RULE: CrateDependencyRule = CrateDependencyRule {
    crate_rel: "apps/frontend",
    forbidden_packages: &["api"],
    reason: "the frontend reaches the server over HTTP, never by linking it",
};

/// The server: the mission and ballistics crates, never a GPU crate or the frontend.
///
/// The GPU crates are the eight that may depend on wgpu (the crate-tier firewall's GPU packages
/// and the `crates/map_rendering` members).
pub const API_RULE: CrateDependencyRule = CrateDependencyRule {
    crate_rel: "apps/api",
    forbidden_packages: &[
        "frontend",
        "gpu_device",
        "gpu_frame",
        "renderer_core",
        "map_renderer",
        "symbology_layers_gpu",
        "world_layers_gpu",
        "map_render_diagnostics",
        "paper_doll_renderer",
    ],
    reason: "the server links the mission and ballistics crates for the mission domain and links \
             neither a GPU crate nor the frontend",
};

/// The offline service worker: cache policy the page also links, never the server, the page or
/// a GPU crate.
pub const OFFLINE_SERVICE_WORKER_RULE: CrateDependencyRule = CrateDependencyRule {
    crate_rel: "apps/offline_service_worker",
    forbidden_packages: &[
        "api",
        "frontend",
        "gpu_device",
        "gpu_frame",
        "renderer_core",
        "map_renderer",
        "symbology_layers_gpu",
        "world_layers_gpu",
        "map_render_diagnostics",
        "paper_doll_renderer",
    ],
    reason: "the offline service worker is a leaf the frontend links; it reaches the server over \
             HTTP and links neither the page nor a GPU crate",
};

/// Every dependency-direction rule of the website applications.
pub const CRATE_DEPENDENCY_RULES: &[CrateDependencyRule] =
    &[FRONTEND_RULE, API_RULE, OFFLINE_SERVICE_WORKER_RULE];

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
