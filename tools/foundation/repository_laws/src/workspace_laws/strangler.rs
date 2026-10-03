//! The strangler law: nothing new depends on a member under `legacy/`, and no member under
//! `legacy/` holds a re-export shim.
//!
//! **Role:** finds every dependency edge from a member outside `legacy/` onto a member under it
//! (apps excepted while `legacy/` exists; no tool depends on it), and every `pub use` of a
//! workspace crate outside `legacy/` inside a member under `legacy/` — a shim, which never
//! survives a commit, whether it re-exports an item, a module or the crate root itself, under its
//! own name or an alias, on one line or across several.
//! **Position:** `cargo xtask verify strangler` prints [`check_strangler`];
//! [`super::crate_tiers`] reports [`legacy_dependency_findings`] as its rule 7;
//! [`super::crate_anatomy`] reads one-line re-exports through [`reexported_crate`].
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** every dependency table counts (normal, dev, build, target-specific); a shim is
//! found on any public re-export statement of the sources of a member under `legacy/`, read from
//! its first line to its `;` with line comments dropped, whatever its shape — `pub use <crate>::…`,
//! `pub use <crate>;`, an alias (`pub use <crate> as x;`), a leading `::`, a group at the top or
//! nested (`pub use {<crate> as x};`, `pub use {{<crate>, x}};`, `pub use <crate>::{self as x};`),
//! a statement spread over lines (`pub use` alone on its line included), and
//! `pub extern crate <crate> as x;` — while restricted visibility (`pub(crate)`, `pub(super)`,
//! `pub(in …)`), a private `use` and line comments re-export nothing; each root of a use tree is
//! judged and the finding cites the statement's first line; such a member whose `src` folder is
//! missing is [`NotRun::TargetMissing`].

use std::path::Path;

use super::crate_layout::{APPS_ROOT, LEGACY_ROOT, is_under};
use super::{LawOutcome, WorkspaceLawReport};
use crate::source_roots::repository_relative;
use crate::workspace_members::{WorkspaceMember, read_workspace_members};
use verification_core::scan;
use verification_core::verdict::NotRun;

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
        let exempt = is_under(&member.path, LEGACY_ROOT) || is_under(&member.path, APPS_ROOT);
        if exempt {
            continue;
        }
        for edge in &member.manifest.dependencies {
            let target = members.iter().find(|m| m.package_name == edge.package);
            if let Some(target) = target.filter(|t| is_under(&t.path, LEGACY_ROOT)) {
                findings.push(format!(
                    "{}/Cargo.toml:{}: [{}] depends on {} — only apps may depend on a member \
                     under legacy/ while legacy/ exists",
                    member.path, edge.line_no, edge.table, target.path
                ));
            }
        }
    }
    findings
}

/// Every public re-export statement naming a workspace crate outside `legacy/` in the sources of
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
                    "{}:{line_no}: shim — a member under legacy/ re-exports `{crate_name}`; \
                     move the callers instead",
                    repository_relative(repo_root, &file),
                ));
            }
        }
    }
    Ok(findings)
}

/// Every crate a public re-export statement of `text` names, with the 1-based first line of its
/// statement: each root of [`reexported_crate`]'s forms, whether the statement sits on one line or
/// spreads over several.
pub fn reexported_crates(text: &str) -> Vec<(usize, String)> {
    reexport_statements(text)
        .into_iter()
        .flat_map(|(line_no, statement)| {
            statement_crates(&statement)
                .into_iter()
                .map(|name| (line_no, name.to_owned()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Every public re-export statement of `text` with its 1-based first line, its lines joined by
/// spaces and their `//` comments dropped, up to its `;` (or the end of the text).
fn reexport_statements(text: &str) -> Vec<(usize, String)> {
    let mut statements = Vec::new();
    let mut open: Option<(usize, String)> = None;
    for (index, line) in text.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        match open.as_mut() {
            Some((_, statement)) => {
                statement.push(' ');
                statement.push_str(code);
            }
            None if public_reexport_tree(code).is_some() => {
                open = Some((index + 1, code.to_string()));
            }
            None => continue,
        }
        if code.contains(';') {
            statements.extend(open.take());
        }
    }
    statements.extend(open);
    statements
}

/// The crate a one-line `pub use <crate>…` re-exports from, the first of its roots; `None` for
/// any other line, `pub(crate) use` included. The crate-anatomy law reads re-exports through it;
/// the strangler law reads whole statements through [`reexported_crates`].
pub fn reexported_crate(line: &str) -> Option<&str> {
    statement_crates(line).into_iter().next()
}

/// Every crate one public re-export statement names as the first segment of a use path: the one
/// path of `pub use <crate>::…;`, `pub use <crate>;`, `pub use <crate> as x;` and
/// `pub use ::<crate> as x;`, each item of a group at the top or nested
/// (`pub use {<crate> as x, {y, z}};`), and the crate of `pub extern crate <crate> as x;`.
/// `crate`, `self`, `super` and `Self` lead no crate; any other statement, restricted visibility
/// included, names none.
fn statement_crates(statement: &str) -> Vec<&str> {
    let Some(tree) = public_reexport_tree(statement) else {
        return Vec::new();
    };
    tree_crates(tree.split(';').next().unwrap_or(""))
}

/// The crates one use tree starts from: one for a path, one per item of a group, nested groups
/// read item by item.
fn tree_crates(tree: &str) -> Vec<&str> {
    let tree = tree.trim_start();
    let tree = tree.strip_prefix("::").unwrap_or(tree).trim_start();
    match tree.strip_prefix('{') {
        Some(group) => top_level_items(group)
            .into_iter()
            .flat_map(tree_crates)
            .collect(),
        None => leading_crate(tree).into_iter().collect(),
    }
}

/// The use tree after `pub use` or `pub extern crate`; `None` for any other text, a restricted
/// visibility such as `pub(crate)` included.
fn public_reexport_tree(text: &str) -> Option<&str> {
    let rest = text.trim_start().strip_prefix("pub")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    if let Some(tree) = after_keyword(rest, "use") {
        return Some(tree);
    }
    after_keyword(after_keyword(rest, "extern")?, "crate")
}

/// The text after `keyword` when it stands as a whole word at the start of `text`, the end of
/// the line included (a statement whose tree starts on the next line).
fn after_keyword<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(keyword)?;
    (rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace() || c == '{' || c == ':'))
        .then(|| rest.trim_start())
}

/// The comma-separated items of a group whose `{` is already consumed, up to its closing `}`;
/// nested groups stay inside their item.
fn top_level_items(group: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, c) in group.char_indices() {
        match c {
            '{' => depth += 1,
            '}' if depth == 0 => {
                items.push(&group[start..index]);
                return items;
            }
            '}' => depth -= 1,
            ',' if depth == 0 => {
                items.push(&group[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    items.push(&group[start..]);
    items
}

/// The first path segment of one use-tree item when it can name a crate.
fn leading_crate(item: &str) -> Option<&str> {
    let item = item.trim_start();
    let item = item.strip_prefix("::").unwrap_or(item).trim_start();
    let end = item
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(item.len());
    let name = &item[..end];
    let rest = item[end..].trim_start();
    let starts_like_a_name = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    let ends_a_segment = rest.is_empty()
        || rest.starts_with("::")
        || rest.starts_with([';', ',', '}'])
        || rest
            .strip_prefix("as")
            .is_some_and(|after| after.starts_with(char::is_whitespace));
    (starts_like_a_name && ends_a_segment && !["crate", "self", "super", "Self"].contains(&name))
        .then_some(name)
}

#[cfg(test)]
#[path = "tests/strangler.rs"]
mod tests;
