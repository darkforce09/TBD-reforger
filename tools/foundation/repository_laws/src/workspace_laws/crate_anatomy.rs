//! The crate-anatomy law: the shape every judged library crate keeps.
//!
//! **Role:** for every library crate in the judged set, checks the manifest half of the anatomy —
//! no `anyhow` dependency; `edition`, `rust-version` and `[lints]` from the workspace; every
//! dependency `workspace = true`; features only `test_fixtures` and `failpoints`, each enabled only
//! by a dev-dependency — and the README's Contents block, then hands the sources to
//! [`super::crate_anatomy_sources`] (`lib.rs`, prelude, `error.rs`, typed ids, re-exports).
//! **Position:** `cargo xtask verify crate-anatomy` prints [`check_crate_anatomy`].
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** a crate is a library when it declares `[lib]` or holds `src/lib.rs`; a crate
//! with only binaries is exempt and named in a note. A missing `lib.rs` or `src` folder of a
//! library crate is [`NotRun::TargetMissing`].

use std::path::Path;

use super::crate_anatomy_sources::source_findings;
use super::crate_layout::is_judged;
use super::{LawOutcome, WorkspaceLawReport};
use crate::cargo_manifest::{DependencyKind, LintsSource};
use crate::workspace_members::{WorkspaceMember, read_workspace_members};
use verification_core::verdict::NotRun;

/// The only features a library crate declares; both exist for tests alone.
pub const ALLOWED_FEATURES: &[&str] = &["test_fixtures", "failpoints"];

/// The `[package]` keys a library crate takes from `[workspace.package]`.
pub const INHERITED_PACKAGE_KEYS: &[&str] = &["edition", "rust-version"];

/// The crate-anatomy report over the checkout at `repo_root`.
pub fn check_crate_anatomy(repo_root: &Path) -> WorkspaceLawReport {
    WorkspaceLawReport::from_outcome("crate-anatomy", crate_anatomy_outcome(repo_root))
}

/// The findings and notes of the crate-anatomy law; [`NotRun`] when an input could not be read.
pub fn crate_anatomy_outcome(repo_root: &Path) -> Result<LawOutcome, NotRun> {
    let members = read_workspace_members(repo_root)?;
    let mut outcome = LawOutcome::default();
    let mut libraries = 0;
    let mut binaries = Vec::new();
    for member in members.iter().filter(|member| is_judged(member)) {
        if !is_library(repo_root, member) {
            binaries.push(member.path.as_str());
            continue;
        }
        libraries += 1;
        outcome.findings.extend(manifest_findings(member, &members));
        outcome.findings.extend(readme_findings(repo_root, member)?);
        outcome
            .findings
            .extend(source_findings(repo_root, member, &members)?);
    }
    outcome.summary = format!(
        "{} workspace member(s), {libraries} judged library crate(s)",
        members.len()
    );
    if !binaries.is_empty() {
        outcome.notes.push(format!(
            "{} judged binary crate(s) exempt: {}",
            binaries.len(),
            binaries.join(", ")
        ));
    }
    Ok(outcome)
}

/// True when `member` builds a library: a `[lib]` table or a `src/lib.rs`.
pub fn is_library(repo_root: &Path, member: &WorkspaceMember) -> bool {
    member.manifest.library.is_some() || repo_root.join(&member.path).join("src/lib.rs").is_file()
}

/// The manifest half of the anatomy for the library crate `member`.
fn manifest_findings(member: &WorkspaceMember, members: &[WorkspaceMember]) -> Vec<String> {
    let manifest = &member.manifest;
    let at = |line_no: usize| format!("{}/Cargo.toml:{line_no}", member.path);
    let mut findings = Vec::new();
    for key in INHERITED_PACKAGE_KEYS {
        match manifest.package_field(key) {
            Some(field) if field.inherited => {}
            Some(field) => findings.push(format!(
                "{}: `{key}` is set locally; take it from the workspace (`{key}.workspace = true`)",
                at(field.line_no)
            )),
            None => findings.push(format!(
                "{}/Cargo.toml: `{key}` is missing; take it from the workspace",
                member.path
            )),
        }
    }
    if manifest.lints != LintsSource::Workspace {
        findings.push(format!(
            "{}/Cargo.toml: lints come from the workspace (`[lints] workspace = true`)",
            member.path
        ));
    }
    for edge in &manifest.dependencies {
        if edge.package == "anyhow" && edge.kind == DependencyKind::Normal {
            findings.push(format!(
                "{}: a library crate never depends on anyhow; return its own Error",
                at(edge.line_no)
            ));
        }
        if !edge.from_workspace {
            findings.push(format!(
                "{}: [{}] {} is not a workspace dependency (`{} = {{ workspace = true }}`)",
                at(edge.line_no),
                edge.table,
                edge.key,
                edge.key
            ));
        }
    }
    findings.extend(feature_findings(member, members));
    findings
}

/// Features other than the dev-only pair, and any way the pair reaches a build that is not a
/// test build.
fn feature_findings(member: &WorkspaceMember, members: &[WorkspaceMember]) -> Vec<String> {
    let package = member.package_name.as_str();
    let mut findings = Vec::new();
    for declared in &member.manifest.features {
        if !ALLOWED_FEATURES.contains(&declared.name.as_str()) {
            findings.push(format!(
                "{}/Cargo.toml:{}: feature `{}` — the only features are {}",
                member.path,
                declared.line_no,
                declared.name,
                ALLOWED_FEATURES.join(" and ")
            ));
        }
    }
    for feature in ALLOWED_FEATURES {
        if member.manifest.feature(feature).is_none() {
            continue;
        }
        for other in members {
            for edge in &other.manifest.dependencies {
                let enables = edge.package == package && edge.features.iter().any(|f| f == feature);
                if enables && !edge.is_dev_dependency() {
                    findings.push(format!(
                        "{}/Cargo.toml:{}: [{}] enables `{package}/{feature}`; only a \
                         dev-dependency may",
                        other.path, edge.line_no, edge.table
                    ));
                }
            }
            for declared in &other.manifest.features {
                let forwards = declared.enables.iter().any(|item| {
                    item == &format!("{package}/{feature}")
                        || item == &format!("{package}?/{feature}")
                        || (other.package_name == package && item == feature)
                });
                if forwards && !ALLOWED_FEATURES.contains(&declared.name.as_str()) {
                    findings.push(format!(
                        "{}/Cargo.toml:{}: feature `{}` enables `{package}/{feature}`, so a build \
                         that is not a test build carries it",
                        other.path, declared.line_no, declared.name
                    ));
                }
            }
        }
    }
    findings
}

/// A README.md with a `## Contents` block in the crate folder.
fn readme_findings(repo_root: &Path, member: &WorkspaceMember) -> Result<Vec<String>, NotRun> {
    let readme = repo_root.join(&member.path).join("README.md");
    if !readme.is_file() {
        return Ok(vec![format!("{}/README.md is missing", member.path)]);
    }
    let text = std::fs::read_to_string(&readme).map_err(|source| NotRun::Unreadable {
        path: readme.clone(),
        source,
    })?;
    let has_contents = text.lines().any(|line| line.trim_end() == "## Contents");
    Ok(if has_contents {
        Vec::new()
    } else {
        vec![format!(
            "{}/README.md has no `## Contents` block",
            member.path
        )]
    })
}

#[cfg(test)]
#[path = "tests/crate_anatomy.rs"]
mod tests;
