//! The strangler law: nothing new depends on a member under `legacy/`, and no member under
//! `legacy/` holds a re-export shim.
//!
//! **Role:** finds every dependency edge from a member outside `legacy/` onto a member under it
//! (apps and the two tool binaries excepted while `legacy/` exists), and every `pub use` of a
//! workspace crate outside `legacy/` inside a member under `legacy/` — a shim, which never
//! survives a commit, whether it re-exports an item, a module or the crate root itself, under its
//! own name or an alias, on one line or across several.
//! **Position:** `cargo xtask verify strangler` prints [`check_strangler`];
//! [`super::crate_tiers`] reports [`legacy_dependency_findings`] as its rule 7.
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** every dependency table counts (normal, dev, build, target-specific); a shim is
//! found on any `pub use` statement of the sources of a member under `legacy/`, read from its
//! `pub use` line to its `;` with line comments dropped, each root of its use tree judged (a
//! grouped root `{…}` names several); the finding cites the `pub use` line; such a member whose
//! `src` folder is missing is [`NotRun::TargetMissing`].

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

/// Every `pub use` statement re-exporting a workspace crate outside `legacy/` in the sources of
/// `member`, which sits under `legacy/`.
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
        for (line_no, crate_name) in reexported_crates(&text) {
            if new_crates.contains(&crate_name) {
                findings.push(format!(
                    "{}:{line_no}: shim — a member under legacy/ re-exports `{crate_name}`; move \
                     the callers instead",
                    repository_relative(repo_root, &file),
                ));
            }
        }
    }
    Ok(findings)
}

/// Every crate a `pub use` statement of `text` re-exports from, with the 1-based line of its
/// `pub use`: `pub use a::Item;`, `pub use a;`, `pub use a as b;`, `pub use ::a::{…};`, a grouped
/// root `pub use {a::X, b as c};`, and any of them spread over several lines. `pub(crate) use`,
/// a private `use` and the roots `crate`, `self` and `super` are no re-export of a crate.
pub fn reexported_crates(text: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let Some(body) = pub_use_body(lines[index]) else {
            index += 1;
            continue;
        };
        let start = index;
        let mut statement = without_line_comment(body).to_owned();
        while !statement.contains(';') && index + 1 < lines.len() {
            index += 1;
            statement.push(' ');
            statement.push_str(without_line_comment(lines[index]));
        }
        let tree = statement.split(';').next().unwrap_or_default();
        found.extend(
            use_tree_roots(tree)
                .into_iter()
                .map(|root| (start + 1, root)),
        );
        index += 1;
    }
    found
}

/// The text after `pub use` when `line` starts a `pub use` statement.
fn pub_use_body(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("pub")?;
    let rest = rest.strip_prefix(char::is_whitespace)?.trim_start();
    let rest = rest.strip_prefix("use")?;
    let starts_tree = rest.is_empty() || rest.starts_with([' ', '\t', '{', ':']);
    starts_tree.then_some(rest)
}

/// `line` up to its `//` comment.
fn without_line_comment(line: &str) -> &str {
    line.find("//").map_or(line, |at| &line[..at])
}

/// The crate names a use tree starts from: one for a path, one per item of a grouped root.
fn use_tree_roots(tree: &str) -> Vec<String> {
    let tree = tree.trim().trim_start_matches("::").trim_start();
    if let Some(group) = tree.strip_prefix('{') {
        let inner = group.rfind('}').map_or(group, |end| &group[..end]);
        return top_level_items(inner)
            .into_iter()
            .flat_map(use_tree_roots)
            .collect();
    }
    let root: String = tree
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if root.is_empty() || ["crate", "self", "super"].contains(&root.as_str()) {
        Vec::new()
    } else {
        vec![root]
    }
}

/// The comma-separated items of a group, nested groups kept whole.
fn top_level_items(group: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut depth = 0_usize;
    let mut item_start = 0;
    for (at, c) in group.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                items.push(&group[item_start..at]);
                item_start = at + 1;
            }
            _ => {}
        }
    }
    items.push(&group[item_start..]);
    items
}

/// The crate a one-line `pub use <crate>::…` re-exports from; `None` for any other line,
/// `pub(crate) use` included. The crate-anatomy law reads re-exports through it; the strangler
/// law reads whole statements through [`reexported_crates`].
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

#[cfg(test)]
#[path = "tests/strangler.rs"]
mod tests;
