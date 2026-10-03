//! Rule 6 of the crate-tier law: the external-crate firewalls.
//!
//! **Role:** keeps each heavy or platform-bound external crate inside the categories built for
//! it: wgpu in the GPU device, frame and core crates, the map renderers and the paper-doll
//! renderer; the browser crates in wasm-only crates, `time_source` behind a target table and the
//! frontend, and never in mission editing — neither as a manifest edge nor as the bare word
//! `web_sys`, `leptos` or `wasm_bindgen` in any `.rs` file under the category; sqlx and axum in
//! api crates, with axum (never sqlx) also in the `tools/browser_testing` and `tools/staging`
//! crates, which are test and staging harness servers rather than product code; leptos in
//! frontend crates; no tokio, axum, reqwest, resvg or image in the dependency closure of xtask
//! (which keeps the harness servers out of it); and no map noun in a declared name of a graphics
//! crate.
//! **Position:** called by [`super::crate_tiers`] over the judged members.
//! **Signals & state:** none; reads parsed manifests, the graphics crates' sources and the mission
//! editing category's sources.
//! **Invariants:** only normal and build edges count (dev-dependencies never ship), an edge is
//! external when no workspace member has its package name, and a graphics crate whose `src`
//! folder is missing is [`NotRun::TargetMissing`]. The mission editing scan runs whenever the
//! category folder exists or a member declares the category; a declared category with no folder
//! is [`NotRun::TargetMissing`], and a folder holding no `.rs` file is a finding, never a pass.

use std::collections::BTreeSet;
use std::path::Path;

use super::crate_layout::{
    CategoryClass, MISSION_EDITING_CATEGORY, TargetPlatforms, category_class, declared_targets,
    effective_category,
};
use crate::cargo_manifest::DependencyEdge;
use crate::engine_layers::MAP_NOUN_DECLARATION_PATTERN;
use crate::workspace_members::WorkspaceMember;
use verification_core::pattern::Pattern;
use verification_core::scan;
use verification_core::verdict::NotRun;

/// Packages that may depend on wgpu outside the map-rendering category.
pub(crate) const GPU_PACKAGES: &[&str] = &[
    "gpu_device",
    "gpu_frame",
    "renderer_core",
    "paper_doll_renderer",
];
/// The browser crates, matched by name or (for `gloo`) by `gloo-` prefix.
pub(crate) const BROWSER_CRATES: &[&str] = &[
    "web-sys",
    "js-sys",
    "wasm-bindgen",
    "wasm-bindgen-futures",
    "gloo",
];
/// The bare words a browser arrives under in Rust source, which no `.rs` file under
/// [`MISSION_EDITING_CATEGORY`] may spell, in code or prose: inside the editing layer a comment
/// telling the next reader to reach for `leptos` is the breach the scan exists to stop. `\b` on
/// both sides, so `web_sysfs` or `leptosaur` is not caught by its prefix.
pub(crate) const BROWSER_SOURCE_TOKEN_PATTERN: &str = r"\b(web_sys|leptos|wasm_bindgen)\b";
/// The one foundation package that may reach the browser, from a target table only.
pub(crate) const TIME_SOURCE_PACKAGE: &str = "time_source";
/// The tool categories whose crates are test and staging harness servers (the gate's static
/// server, the staging relay): axum, never sqlx, is allowed there; [`XTASK_CLOSURE_BANS`] keeps
/// them out of xtask.
pub(crate) const HARNESS_SERVER_CATEGORIES: &[&str] = &["tools/browser_testing", "tools/staging"];
/// External crates that never enter the dependency closure of xtask.
pub(crate) const XTASK_CLOSURE_BANS: &[&str] = &["tokio", "axum", "reqwest", "resvg", "image"];
/// The package name of the xtask binary.
pub(crate) const XTASK_PACKAGE: &str = "xtask";

/// Every rule-6 finding over the judged members.
pub(super) fn firewall_findings(
    repo_root: &Path,
    members: &[WorkspaceMember],
    judged: &[&WorkspaceMember],
) -> Result<Vec<String>, NotRun> {
    let is_member = |package: &str| members.iter().any(|m| m.package_name == package);
    let mut findings = Vec::new();
    for member in judged {
        let category = effective_category(member);
        let class = category_class(&category);
        for edge in shipped_edges(member).filter(|edge| !is_member(&edge.package)) {
            if let Some(rule) = breached_firewall(member, &category, class, edge) {
                findings.push(format!(
                    "rule 6: {}/Cargo.toml:{}: {} — {rule}",
                    member.path, edge.line_no, edge.package
                ));
            }
        }
        if member.package_name == XTASK_PACKAGE {
            findings.extend(xtask_closure_findings(member, members));
        }
        if class == Some(CategoryClass::Graphics) {
            findings.extend(map_noun_findings(repo_root, member)?);
        }
    }
    findings.extend(mission_editing_browser_token_findings(repo_root, members)?);
    Ok(findings)
}

/// The normal and build edges of `member`.
fn shipped_edges(member: &WorkspaceMember) -> impl Iterator<Item = &DependencyEdge> {
    member
        .manifest
        .dependencies
        .iter()
        .filter(|edge| !edge.is_dev_dependency())
}

/// The firewall `edge` breaches, when it breaches one.
fn breached_firewall(
    member: &WorkspaceMember,
    category: &str,
    class: Option<CategoryClass>,
    edge: &DependencyEdge,
) -> Option<&'static str> {
    let name = edge.package.as_str();
    let package = member.package_name.as_str();
    if name == "wgpu" || name.starts_with("wgpu-") {
        let allowed = category == "crates/map_rendering" || GPU_PACKAGES.contains(&package);
        return (!allowed).then_some(
            "wgpu lives only in the GPU device, frame and core crates and the renderers",
        );
    }
    if BROWSER_CRATES.contains(&name) || name.starts_with("gloo-") {
        if class == Some(CategoryClass::MissionEditing) {
            return Some("mission editing names no browser crate");
        }
        let allowed = declared_targets(member) == Some(TargetPlatforms::Wasm32)
            || class == Some(CategoryClass::Frontend)
            || (package == TIME_SOURCE_PACKAGE && edge.target_cfg.is_some());
        return (!allowed).then_some(
            "browser crates live only in wasm-only crates, time_source behind a target table and \
             the frontend",
        );
    }
    let is_crate =
        |crate_name: &str| name == crate_name || name.starts_with(&format!("{crate_name}-"));
    if is_crate("sqlx") || is_crate("axum") {
        let allowed = class == Some(CategoryClass::Api)
            || (is_crate("axum") && HARNESS_SERVER_CATEGORIES.contains(&category));
        return (!allowed).then_some(
            "sqlx and axum live only in api crates; axum also in the browser_testing and staging \
             harness servers",
        );
    }
    if name == "leptos" || name.starts_with("leptos_") || name.starts_with("leptos-") {
        return (class != Some(CategoryClass::Frontend))
            .then_some("leptos lives only in frontend crates");
    }
    None
}

/// The banned crates in the dependency closure of xtask, each with the member that brings it.
fn xtask_closure_findings(xtask: &WorkspaceMember, members: &[WorkspaceMember]) -> Vec<String> {
    let mut visited: BTreeSet<&str> = BTreeSet::new();
    let mut pending = vec![xtask];
    let mut findings = Vec::new();
    while let Some(member) = pending.pop() {
        if !visited.insert(member.package_name.as_str()) {
            continue;
        }
        for edge in shipped_edges(member) {
            match members.iter().find(|m| m.package_name == edge.package) {
                Some(dependency) => pending.push(dependency),
                None if XTASK_CLOSURE_BANS.contains(&edge.package.as_str()) => {
                    findings.push(format!(
                        "rule 6: {}/Cargo.toml:{}: {} enters the dependency closure of xtask",
                        member.path, edge.line_no, edge.package
                    ));
                }
                None => {}
            }
        }
    }
    findings
}

/// Every line spelling [`BROWSER_SOURCE_TOKEN_PATTERN`] in a `.rs` file under
/// [`MISSION_EDITING_CATEGORY`], or one finding when that folder holds no `.rs` file at all: "no
/// source names a browser" and "there is no source" read alike to a matcher, so a walk of zero
/// files is refused rather than passed. A checkout with neither the folder nor a member declaring
/// the category has no mission editing layer to judge.
fn mission_editing_browser_token_findings(
    repo_root: &Path,
    members: &[WorkspaceMember],
) -> Result<Vec<String>, NotRun> {
    let root = repo_root.join(MISSION_EDITING_CATEGORY);
    let declared = members
        .iter()
        .any(|member| effective_category(member) == MISSION_EDITING_CATEGORY);
    if !declared && !root.exists() {
        return Ok(Vec::new());
    }
    let files = scan::walk_files(&[root.as_path()], scan::with_extension(&["rs"]))?;
    if files.is_empty() {
        return Ok(vec![format!(
            "rule 6: walked 0 .rs file(s) under {MISSION_EDITING_CATEGORY} — the mission editing \
             browser scan refuses a vacuous pass"
        )]);
    }
    let pattern =
        Pattern::regex(BROWSER_SOURCE_TOKEN_PATTERN).expect("the browser token pattern compiles");
    Ok(scan::matching_lines(&pattern, &files)?
        .iter()
        .map(|hit| {
            format!(
                "rule 6: {}:{}: mission editing names no browser crate, prose included: {}",
                crate::source_roots::repository_relative(repo_root, &hit.path),
                hit.line_no,
                hit.line.trim()
            )
        })
        .collect())
}

/// Every declared name with a map noun in the sources of the graphics crate `member`.
fn map_noun_findings(repo_root: &Path, member: &WorkspaceMember) -> Result<Vec<String>, NotRun> {
    let pattern = Pattern::regex(MAP_NOUN_DECLARATION_PATTERN)
        .and_then(Pattern::case_insensitive)
        .expect("the map-noun pattern compiles");
    let source = repo_root.join(&member.path).join("src");
    let files = scan::walk_files(&[source.as_path()], scan::with_extension(&["rs"]))?;
    let mut findings = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).map_err(|source| NotRun::Unreadable {
            path: file.clone(),
            source,
        })?;
        for (index, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if !code.starts_with("//") && pattern.is_match(code) {
                findings.push(format!(
                    "rule 6: {}:{}: a graphics crate declares no map noun: {}",
                    crate::source_roots::repository_relative(repo_root, &file),
                    index + 1,
                    code
                ));
            }
        }
    }
    Ok(findings)
}
