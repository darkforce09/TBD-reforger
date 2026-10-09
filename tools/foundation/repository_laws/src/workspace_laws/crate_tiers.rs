//! The crate-tier law: membership, layout declarations, the tier order and the category matrix.
//!
//! **Role:** judges rules 1–7 of the crate-tier law over a checkout: every manifest under the
//! sweep roots is a workspace member (1); each judged member declares its layout, and every member
//! outside the judged set is a tool binary (2); each judged member sits at its category plus its
//! name (3); tiers are recomputed from dependencies and edges point strictly down (4); the
//! category edge matrix and the wasm-only edge rule hold (5); the firewalls hold (6,
//! `super::crate_firewalls`); and no member depends on an application package, in any table (7).
//! **Position:** `cargo xtask verify crate-tiers` prints [`check_crate_tiers`]; xtask passes the
//! [`CrateTierConfiguration`] (the sweep roots and the application packages). Reads
//! [`crate::workspace_members`].
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** a member's tier is 0 with no judged workspace dependency and otherwise 1 plus
//! the highest tier among them, over normal and build edges in every target table; the declared
//! tier must equal it. A sweep root that does not exist holds no manifest and is named in a note.
//! The tool binaries ([`super::crate_layout::is_tool_binary`]) are listed in a note; any other
//! member outside the judged set is a rule 2 finding. Rule 7 reads every member's every table —
//! normal, build, dev and target-specific — under each edge's real package name, a crate's edge
//! onto itself excepted; an application package that is no member is a rule 7 finding, never a
//! pass.

use std::collections::HashMap;
use std::path::Path;

use super::crate_layout::{
    EdgeEnd, TOOL_BINARY_PATHS, TargetPlatforms, category_class, category_edge_allowed,
    declared_targets, effective_category, is_judged, is_tool_binary, is_wasm32_only_cfg,
};
use super::{LawOutcome, WorkspaceLawReport, crate_firewalls};
use crate::workspace_members::{WorkspaceMember, child_folder_names, read_workspace_members};
use verification_core::verdict::NotRun;

/// Folder names the stray-manifest sweep does not enter: test trees, fixtures, build output and
/// vendored npm packages hold manifests that are not crates of this workspace.
pub const SWEEP_SKIPPED_FOLDERS: &[&str] = &[
    "tests",
    "fixtures",
    "test_fixtures",
    "target",
    "node_modules",
];

/// What the crate-tier law reads that moves with the tree; xtask passes it in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrateTierConfiguration<'a> {
    /// Repository-relative folders whose every manifest must be a member (rule 1).
    pub manifest_sweep_roots: &'a [&'a str],
    /// The application packages, by package name: each must be a member, and no member depends
    /// on one in any table (rule 7).
    pub application_packages: &'a [&'a str],
}

/// The crate-tier report over the checkout at `repo_root` under `configuration`.
pub fn check_crate_tiers(
    repo_root: &Path,
    configuration: &CrateTierConfiguration<'_>,
) -> WorkspaceLawReport {
    WorkspaceLawReport::from_outcome("crate-tiers", crate_tier_outcome(repo_root, configuration))
}

/// The findings and notes of the crate-tier law; [`NotRun`] when an input could not be read.
pub fn crate_tier_outcome(
    repo_root: &Path,
    configuration: &CrateTierConfiguration<'_>,
) -> Result<LawOutcome, NotRun> {
    let manifest_sweep_roots = configuration.manifest_sweep_roots;
    let members = read_workspace_members(repo_root)?;
    let judged: Vec<&WorkspaceMember> = members.iter().filter(|m| is_judged(m)).collect();
    let mut outcome = LawOutcome {
        summary: format!(
            "{} workspace member(s), {} judged",
            members.len(),
            judged.len()
        ),
        ..LawOutcome::default()
    };
    let sweep = sweep_manifests(repo_root, manifest_sweep_roots)?;
    outcome.notes.push(sweep.note(manifest_sweep_roots));
    for manifest_folder in &sweep.folders {
        if !members.iter().any(|member| &member.path == manifest_folder) {
            outcome.findings.push(format!(
                "rule 1: {manifest_folder}/Cargo.toml is not a workspace member — add the folder \
                 to the root [workspace] members"
            ));
        }
    }
    for member in &judged {
        outcome.findings.extend(declaration_findings(member));
    }
    outcome.findings.extend(edge_findings(&members, &judged));
    outcome.findings.extend(application_edge_findings(
        &members,
        configuration.application_packages,
    ));
    outcome.findings.extend(crate_firewalls::firewall_findings(
        repo_root, &members, &judged,
    )?);
    let (allowed, unjudged): (Vec<&str>, Vec<&str>) = members
        .iter()
        .filter(|member| !is_judged(member))
        .map(|member| member.path.as_str())
        .partition(|path| is_tool_binary(path));
    if !allowed.is_empty() {
        outcome.notes.push(format!(
            "{} member(s) outside the judged set, each a tool binary: {}",
            allowed.len(),
            allowed.join(", ")
        ));
    }
    for path in unjudged {
        outcome.findings.push(format!(
            "rule 2: {path} is a member outside the judged set — move it to \
             crates/<category>/<name> or tools/<category>/<name> with a \
             [package.metadata.layout] table; only the tool binaries ({}) stay outside",
            TOOL_BINARY_PATHS.join(", ")
        ));
    }
    Ok(outcome)
}

/// Rule 7 over every member: no dependency edge, in any table, names an application package (a
/// crate's edge onto itself excepted), and every application package is a member.
fn application_edge_findings(
    members: &[WorkspaceMember],
    application_packages: &[&str],
) -> Vec<String> {
    let mut findings: Vec<String> = application_packages
        .iter()
        .filter(|package| !members.iter().any(|m| m.package_name == **package))
        .map(|package| {
            format!(
                "rule 7: application package `{package}` is no workspace member — the \
                 application list names members only"
            )
        })
        .collect();
    for member in members {
        for edge in &member.manifest.dependencies {
            if edge.package != member.package_name
                && application_packages.contains(&edge.package.as_str())
            {
                findings.push(format!(
                    "rule 7: {}/Cargo.toml:{}: [{}] depends on the application {} — no member \
                     depends on an application package, in any table",
                    member.path, edge.line_no, edge.table, edge.package
                ));
            }
        }
    }
    findings
}

/// The manifests the sweep found.
struct ManifestSweep {
    /// Repository-relative folders holding a `Cargo.toml`.
    folders: Vec<String>,
    /// Sweep roots that do not exist.
    absent_roots: Vec<String>,
}

impl ManifestSweep {
    /// The note line naming what the sweep read.
    fn note(&self, roots: &[&str]) -> String {
        let absent = if self.absent_roots.is_empty() {
            String::new()
        } else {
            format!(" ({} absent)", self.absent_roots.join(", "))
        };
        format!(
            "swept {} manifest(s) under {}{absent}",
            self.folders.len(),
            roots.join(", ")
        )
    }
}

/// Every folder under `roots` that holds a `Cargo.toml`, outside [`SWEEP_SKIPPED_FOLDERS`].
fn sweep_manifests(repo_root: &Path, roots: &[&str]) -> Result<ManifestSweep, NotRun> {
    let mut sweep = ManifestSweep {
        folders: Vec::new(),
        absent_roots: Vec::new(),
    };
    let mut pending: Vec<String> = Vec::new();
    for root in roots {
        if repo_root.join(root).is_dir() {
            pending.push(root.to_string());
        } else {
            sweep.absent_roots.push(root.to_string());
        }
    }
    while let Some(folder) = pending.pop() {
        let absolute = repo_root.join(&folder);
        if absolute.join("Cargo.toml").is_file() {
            sweep.folders.push(folder.clone());
        }
        for name in child_folder_names(&absolute)? {
            if !SWEEP_SKIPPED_FOLDERS.contains(&name.as_str()) {
                pending.push(format!("{folder}/{name}"));
            }
        }
    }
    sweep.folders.sort();
    Ok(sweep)
}

/// Rules 2 and 3 for one judged member.
fn declaration_findings(member: &WorkspaceMember) -> Vec<String> {
    let path = &member.path;
    let Some(layout) = member.manifest.layout.as_ref() else {
        return vec![format!(
            "rule 2: {path} declares no [package.metadata.layout] (category, tier, targets)"
        )];
    };
    let mut findings = Vec::new();
    match layout.category.as_deref() {
        None => findings.push(format!("rule 2: {path} declares no layout category")),
        Some(category) => {
            if category != member.parent_folder() {
                findings.push(format!(
                    "rule 3: {path} declares category `{category}` but sits in `{}`",
                    member.parent_folder()
                ));
            }
            if category_class(category).is_none() {
                findings.push(format!(
                    "rule 3: {path} declares unknown category `{category}`"
                ));
            }
        }
    }
    if layout.tier_number().is_none() {
        findings.push(format!(
            "rule 2: {path} declares tier `{}`, not a whole number",
            layout.tier.as_deref().unwrap_or("")
        ));
    }
    if declared_targets(member).is_none() {
        findings.push(format!(
            "rule 2: {path} declares targets `{}`; the values are `any` and `wasm32`",
            layout.targets.as_deref().unwrap_or("")
        ));
    }
    if member.package_name != member.folder_name() {
        findings.push(format!(
            "rule 3: {path} is package `{}`; the package name equals the folder name `{}`",
            member.package_name,
            member.folder_name()
        ));
    }
    findings
}

/// Rules 4 and 5 over every normal and build edge of the judged members.
fn edge_findings(members: &[WorkspaceMember], judged: &[&WorkspaceMember]) -> Vec<String> {
    let by_package: HashMap<&str, &WorkspaceMember> = members
        .iter()
        .map(|member| (member.package_name.as_str(), member))
        .collect();
    let judged_by_package: HashMap<&str, &WorkspaceMember> = judged
        .iter()
        .map(|member| (member.package_name.as_str(), *member))
        .collect();
    let mut memo = HashMap::new();
    let mut findings = Vec::new();
    for member in judged {
        let declared = member
            .manifest
            .layout
            .as_ref()
            .and_then(|l| l.tier_number());
        let computed = computed_tier(member, &judged_by_package, &mut memo, &mut Vec::new());
        if let Some(declared) = declared
            && declared != computed
        {
            findings.push(format!(
                "rule 4: {} declares tier {declared}; its dependencies make it tier {computed}",
                member.path
            ));
        }
        let category = effective_category(member);
        for edge in &member.manifest.dependencies {
            let Some(target) = by_package.get(edge.package.as_str()) else {
                continue;
            };
            if target.package_name == member.package_name {
                continue;
            }
            if edge.is_dev_dependency() {
                continue;
            }
            let at = format!("{}/Cargo.toml:{}", member.path, edge.line_no);
            let target_tier = target
                .manifest
                .layout
                .as_ref()
                .and_then(|l| l.tier_number());
            if let (Some(own), Some(theirs)) = (declared, target_tier)
                && theirs >= own
            {
                findings.push(format!(
                    "rule 4: {at}: tier {own} depends on {} at tier {theirs}; edges point \
                     strictly down",
                    target.path
                ));
            }
            let target_category = effective_category(target);
            let from = EdgeEnd {
                path: &member.path,
                category: &category,
                targets: declared_targets(member),
            };
            let to = EdgeEnd {
                path: &target.path,
                category: &target_category,
                targets: if is_judged(target) {
                    declared_targets(target)
                } else {
                    None
                },
            };
            let target_category_label = if is_judged(target) {
                target_category.as_str()
            } else {
                "outside the layout"
            };
            if !category_edge_allowed(from, to) {
                findings.push(format!(
                    "rule 5: {at}: {category} may not depend on {} ({target_category_label})",
                    target.path
                ));
            }
            if to.targets == Some(TargetPlatforms::Wasm32)
                && from.targets == Some(TargetPlatforms::Any)
                && !edge.target_cfg.as_deref().is_some_and(is_wasm32_only_cfg)
            {
                findings.push(format!(
                    "rule 5: {at}: wasm-only {} is reached from [{}]; a crate for every platform \
                     reaches it only from [target.'cfg(target_arch = \"wasm32\")'.dependencies]",
                    target.path, edge.table
                ));
            }
        }
    }
    findings
}

/// 0 with no judged workspace dependency, otherwise 1 plus the highest dependency tier, over
/// normal and build edges. A cycle (which Cargo refuses to build) contributes nothing.
fn computed_tier<'a>(
    member: &'a WorkspaceMember,
    judged: &HashMap<&'a str, &'a WorkspaceMember>,
    memo: &mut HashMap<&'a str, u32>,
    visiting: &mut Vec<&'a str>,
) -> u32 {
    let name = member.package_name.as_str();
    if let Some(tier) = memo.get(name) {
        return *tier;
    }
    if visiting.contains(&name) {
        return 0;
    }
    visiting.push(name);
    let mut tier = 0;
    for edge in &member.manifest.dependencies {
        if edge.is_dev_dependency() || edge.package == name {
            continue;
        }
        if let Some(dependency) = judged.get(edge.package.as_str()) {
            tier = tier.max(1 + computed_tier(dependency, judged, memo, visiting));
        }
    }
    visiting.pop();
    memo.insert(name, tier);
    tier
}

#[cfg(test)]
#[path = "tests/crate_tiers.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/crate_tiers_application_boundaries.rs"]
mod application_boundary_tests;
