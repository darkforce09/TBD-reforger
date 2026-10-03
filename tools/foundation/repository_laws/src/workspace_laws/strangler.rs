//! The strangler law: nothing new depends on a member under `legacy/`, and no member under
//! `legacy/` holds a re-export shim.
//!
//! **Role:** finds every dependency edge from a member outside `legacy/` onto a member under it
//! (apps and the two tool binaries excepted while `legacy/` exists), and every `pub use` of a
//! workspace crate outside `legacy/` inside a member under `legacy/` — a shim, which never
//! survives a commit.
//! **Position:** `cargo xtask verify strangler` prints [`check_strangler`];
//! [`super::crate_tiers`] reports [`legacy_dependency_findings`] as its rule 7.
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** every dependency table counts (normal, dev, build, target-specific), a shim is
//! found on any `pub use` line of the sources of a member under `legacy/`, and such a member
//! whose `src` folder is missing is [`NotRun::TargetMissing`].

use std::path::Path;

use super::crate_layout::{APPS_ROOT, LEGACY_ROOT, is_under};
use super::{LawOutcome, WorkspaceLawReport};
use crate::source_roots::repository_relative;
use crate::workspace_members::{WorkspaceMember, read_workspace_members};
use verification_core::scan;
use verification_core::verdict::NotRun;

/// The two tool binaries that may depend on members under `legacy/` while `legacy/` exists.
pub const LEGACY_DEPENDENT_TOOL_BINARIES: &[&str] = &["tools/xtask", "tools/developer_tools"];

/// The strangler report over the checkout at `repo_root`.
pub fn check_strangler(repo_root: &Path) -> WorkspaceLawReport {
    WorkspaceLawReport::from_outcome("strangler", strangler_outcome(repo_root))
}

/// The findings of the strangler law; [`NotRun`] when an input could not be read.
pub fn strangler_outcome(repo_root: &Path) -> Result<LawOutcome, NotRun> {
    let members = read_workspace_members(repo_root)?;
    let parked_members: Vec<&WorkspaceMember> = members
        .iter()
        .filter(|member| is_under(&member.path, LEGACY_ROOT))
        .collect();
    let mut findings = legacy_dependency_findings(&members);
    for member in &parked_members {
        findings.extend(shim_findings(repo_root, member, &members)?);
    }
    Ok(LawOutcome {
        summary: format!(
            "{} workspace member(s), {} under {LEGACY_ROOT}/",
            members.len(),
            parked_members.len()
        ),
        findings,
        notes: Vec::new(),
    })
}

/// Every edge from a member that may not depend on `legacy/` onto a member under `legacy/`.
pub fn legacy_dependency_findings(members: &[WorkspaceMember]) -> Vec<String> {
    let mut findings = Vec::new();
    for member in members {
        let exempt = is_under(&member.path, LEGACY_ROOT)
            || is_under(&member.path, APPS_ROOT)
            || LEGACY_DEPENDENT_TOOL_BINARIES.contains(&member.path.as_str());
        if exempt {
            continue;
        }
        for edge in &member.manifest.dependencies {
            let target = members.iter().find(|m| m.package_name == edge.package);
            if let Some(target) = target.filter(|t| is_under(&t.path, LEGACY_ROOT)) {
                findings.push(format!(
                    "{}/Cargo.toml:{}: [{}] depends on {} — only apps and the tool binaries \
                     may depend on a member under legacy/ while legacy/ exists",
                    member.path, edge.line_no, edge.table, target.path
                ));
            }
        }
    }
    findings
}

/// Every `pub use <workspace crate outside legacy/>::` line in the sources of `member`, which
/// sits under `legacy/`.
fn shim_findings(
    repo_root: &Path,
    member: &WorkspaceMember,
    members: &[WorkspaceMember],
) -> Result<Vec<String>, NotRun> {
    let new_crates: Vec<String> = members
        .iter()
        .filter(|m| !is_under(&m.path, LEGACY_ROOT))
        .map(WorkspaceMember::crate_identifier)
        .collect();
    let source = repo_root.join(&member.path).join("src");
    let files = scan::walk_files(&[source.as_path()], scan::with_extension(&["rs"]))?;
    let mut findings = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).map_err(|source| NotRun::Unreadable {
            path: file.clone(),
            source,
        })?;
        for (index, line) in text.lines().enumerate() {
            if let Some(crate_name) = reexported_crate(line)
                && new_crates.iter().any(|name| name == crate_name)
            {
                findings.push(format!(
                    "{}:{}: shim — a member under legacy/ re-exports `{crate_name}`; move the \
                     callers instead",
                    repository_relative(repo_root, &file),
                    index + 1
                ));
            }
        }
    }
    Ok(findings)
}

/// The crate a `pub use <crate>::…` line re-exports from; `None` for any other line,
/// `pub(crate) use` included.
pub fn reexported_crate(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("pub use ")?;
    let rest = rest.trim_start().trim_start_matches("::");
    let end = rest.find("::")?;
    let name = &rest[..end];
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
        .then_some(name)
        .filter(|name| !["crate", "self", "super"].contains(name))
}
