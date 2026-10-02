//! The crate-tier law: membership, layout declarations, the tier order and the category matrix.
//!
//! **Role:** judges rules 1–8 of the crate-tier law over a checkout: every manifest under the
//! sweep roots is a workspace member (1); each judged member declares its layout (2) and sits at
//! its category plus its name (3); tiers are recomputed from dependencies and edges point strictly
//! down (4); the category edge matrix and the wasm-only edge rule hold (5); the firewalls hold (6,
//! [`super::crate_firewalls`]); nothing new depends on a member under `legacy/` (7,
//! [`super::strangler`]); and dev-dependencies never point at `apps/` or `legacy/` (8).
//! **Position:** `cargo xtask verify crate-tiers` prints [`check_crate_tiers`]; xtask passes the
//! sweep roots. Reads [`crate::repository_laws::workspace_members`].
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** a member's tier is 0 with no judged workspace dependency and otherwise 1 plus
//! the highest tier among them, over normal and build edges in every target table; the declared
//! tier must equal it. A sweep root that does not exist holds no manifest and is named in a note.
//! Members outside the judged set are listed in a note while
//! [`super::crate_layout::UNJUDGED_MEMBERS_FAIL`] is false.

use std::collections::HashMap;
use std::path::Path;

use super::crate_layout::{
    APPS_ROOT, EdgeEnd, LEGACY_ROOT, TargetPlatforms, UNJUDGED_MEMBERS_FAIL, category_class,
    category_edge_allowed, declared_targets, effective_category, is_judged, is_under,
    is_wasm32_only_cfg,
};
use super::{LawOutcome, WorkspaceLawReport, crate_firewalls, strangler};
use crate::repository_laws::workspace_members::{
    WorkspaceMember, child_folder_names, read_workspace_members,
};
use crate::verdict::NotRun;

/// Folder names the stray-manifest sweep does not enter: test trees, fixtures, build output and
/// vendored npm packages hold manifests that are not crates of this workspace.
pub const SWEEP_SKIPPED_FOLDERS: &[&str] = &[
    "tests",
    "fixtures",
    "test_fixtures",
    "target",
    "node_modules",
];

/// The crate-tier report over the checkout at `repo_root`, sweeping `manifest_sweep_roots`
/// (repository-relative folders) for manifests that are not members.
pub fn check_crate_tiers(repo_root: &Path, manifest_sweep_roots: &[&str]) -> WorkspaceLawReport {
    WorkspaceLawReport::from_outcome(
        "crate-tiers",
        crate_tier_outcome(repo_root, manifest_sweep_roots),
    )
}

/// The findings and notes of the crate-tier law; [`NotRun`] when an input could not be read.
pub fn crate_tier_outcome(
    repo_root: &Path,
    manifest_sweep_roots: &[&str],
) -> Result<LawOutcome, NotRun> {
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
    outcome.findings.extend(crate_firewalls::firewall_findings(
        repo_root, &members, &judged,
    )?);
    outcome.findings.extend(
        strangler::legacy_dependency_findings(&members)
            .into_iter()
            .map(|f| format!("rule 7: {f}")),
    );
    let unjudged: Vec<&str> = members
        .iter()
        .filter(|member| !is_judged(member))
        .map(|member| member.path.as_str())
        .collect();
    if !unjudged.is_empty() {
        let line = format!(
            "{} member(s) outside the judged set: {}",
            unjudged.len(),
            unjudged.join(", ")
        );
        if UNJUDGED_MEMBERS_FAIL {
            outcome.findings.push(format!("rule 2: {line}"));
        } else {
            outcome.notes.push(line);
        }
    }
    Ok(outcome)
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

/// Rules 4, 5 and 8 over every dependency edge of the judged members.
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
            let at = format!("{}/Cargo.toml:{}", member.path, edge.line_no);
            if edge.is_dev_dependency() {
                if is_under(&target.path, APPS_ROOT) || is_under(&target.path, LEGACY_ROOT) {
                    findings.push(format!(
                        "rule 8: {at}: dev-dependency on {} — dev-dependencies never point at \
                         apps/ or legacy/",
                        target.path
                    ));
                }
                continue;
            }
            if is_under(&target.path, LEGACY_ROOT) {
                continue;
            }
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
