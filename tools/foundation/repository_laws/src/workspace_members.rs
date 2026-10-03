//! The workspace member reader: which crates the root manifest names, and their manifests.
//!
//! **Role:** reads the root `Cargo.toml` `[workspace]` table, expands its `members` entries —
//! explicit folders and globs such as `crates/*/*` — minus its `exclude` entries, and returns each
//! member's repository-relative folder, package name and parsed manifest.
//! **Position:** the shared input of the workspace laws in [`super::workspace_laws`]; built on
//! [`super::cargo_manifest`].
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** a missing root manifest, a root manifest with no `[workspace]` table, an
//! explicit member folder that is missing and a member without a `Cargo.toml` are
//! [`NotRun::TargetMissing`], never a smaller workspace. A glob matches folders only, a matched
//! folder without a `Cargo.toml` is not a member (Cargo reads the same glob the same way), and the
//! result is sorted by path so every report over it is deterministic.

use std::path::Path;

use super::cargo_manifest::{CargoManifest, read_manifest};
use verification_core::verdict::NotRun;

/// One member of the workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMember {
    /// The member's folder, repository-relative with `/` separators.
    pub path: String,
    /// `[package] name`; empty when the manifest declares none.
    pub package_name: String,
    /// The member's parsed manifest.
    pub manifest: CargoManifest,
}

impl WorkspaceMember {
    /// The member's folder name: the last component of [`WorkspaceMember::path`].
    pub fn folder_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or_default()
    }

    /// The folder path between the repository root and the member's folder; empty for a member
    /// at the root.
    pub fn parent_folder(&self) -> &str {
        self.path.rsplit_once('/').map_or("", |(parent, _)| parent)
    }

    /// The name Rust code spells the package with: its package name with `-` as `_`.
    pub fn crate_identifier(&self) -> String {
        self.package_name.replace('-', "_")
    }
}

/// Every member of the workspace whose root manifest is `repo_root/Cargo.toml`, sorted by path.
pub fn read_workspace_members(repo_root: &Path) -> Result<Vec<WorkspaceMember>, NotRun> {
    let root_manifest_path = repo_root.join("Cargo.toml");
    let root_manifest = read_manifest(&root_manifest_path)?;
    let Some(workspace) = root_manifest.workspace else {
        return Err(NotRun::TargetMissing(root_manifest_path));
    };
    let excluded: Vec<String> = workspace.exclude.iter().map(|e| normalized(e)).collect();
    let mut paths: Vec<String> = Vec::new();
    for entry in &workspace.members {
        let entry = normalized(entry);
        if entry.contains(['*', '?']) {
            paths.extend(expand_member_glob(repo_root, &entry)?);
        } else {
            let folder = repo_root.join(&entry);
            if !folder.is_dir() {
                return Err(NotRun::TargetMissing(folder));
            }
            paths.push(entry);
        }
    }
    paths.retain(|path| !excluded.contains(path));
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .map(|path| {
            let manifest = read_manifest(&repo_root.join(&path).join("Cargo.toml"))?;
            Ok(WorkspaceMember {
                package_name: manifest.package_name.clone().unwrap_or_default(),
                path,
                manifest,
            })
        })
        .collect()
}

/// `entry` with `\` as `/` and no leading `./` or trailing `/`.
fn normalized(entry: &str) -> String {
    entry
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

/// Every folder under `repo_root` that `pattern` matches segment by segment and that holds a
/// `Cargo.toml`. A folder level that does not exist matches nothing.
fn expand_member_glob(repo_root: &Path, pattern: &str) -> Result<Vec<String>, NotRun> {
    let mut matched: Vec<String> = vec![String::new()];
    for segment in pattern.split('/') {
        let mut next = Vec::new();
        for prefix in &matched {
            let folder = repo_root.join(prefix);
            if !segment.contains(['*', '?']) {
                if folder.join(segment).is_dir() {
                    next.push(joined(prefix, segment));
                }
                continue;
            }
            if !folder.is_dir() {
                continue;
            }
            for name in child_folder_names(&folder)? {
                if wildcard_matches(segment, &name) {
                    next.push(joined(prefix, &name));
                }
            }
        }
        matched = next;
    }
    matched.retain(|path| repo_root.join(path).join("Cargo.toml").is_file());
    Ok(matched)
}

/// The names of the folders directly inside `folder`, hidden folders left out.
pub(super) fn child_folder_names(folder: &Path) -> Result<Vec<String>, NotRun> {
    let unreadable = |source| NotRun::Unreadable {
        path: folder.to_path_buf(),
        source,
    };
    let mut names = Vec::new();
    for entry in std::fs::read_dir(folder).map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with('.') && entry.file_type().map_err(unreadable)?.is_dir() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

/// `prefix/name`, or `name` when `prefix` is empty.
fn joined(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    }
}

/// True when `name` matches the glob segment `pattern`, where `*` matches any run of characters
/// and `?` any one character.
pub fn wildcard_matches(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();
    let (mut p, mut n) = (0, 0);
    let mut backtrack: Option<(usize, usize)> = None;
    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            backtrack = Some((p, n));
            p += 1;
        } else if let Some((star, matched_to)) = backtrack {
            p = star + 1;
            n = matched_to + 1;
            backtrack = Some((star, matched_to + 1));
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|c| *c == '*')
}

#[cfg(test)]
#[path = "tests/workspace_members.rs"]
mod tests;
