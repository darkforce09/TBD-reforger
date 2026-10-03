//! The stage manifests in the order they entered the history.
//!
//! **Role:** lists every stage manifest of the manifests folder (every `.tsv` but the format
//! sample) and orders them chronologically, so `--verify` can follow an earlier manifest's scopes
//! through the moves of the manifests after it ([`super::scope_history`]).
//!
//! **Position:** read by `cargo xtask refactor relocate --verify`
//! ([`super::relocation_modes::verify`]); the order comes from `git log` over the manifests folder.
//!
//! **Signals & state:** none; one directory listing and two or three git queries per call.
//!
//! **Invariants:** a manifest's place is the first commit of `HEAD`'s history that added it
//! (`git log --topo-order --reverse --no-renames --diff-filter=A`), oldest first; manifests one
//! commit added sort by file name; manifests no commit added (untracked, or staged but not
//! committed) come after every committed one, by file name; a checkout without a commit orders
//! every manifest by file name; a shallow clone, whose history is cut, and a git query that fails
//! are a [`NotRun`], never a guessed order.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use verification_core::NotRun;

use super::file_treatment::manifests_folder;
use super::relocation_modes::EXAMPLE_MANIFEST;
use super::repository_files::{git, has_head_commit};

/// The line `git log` prints before the files of each commit.
const COMMIT_MARKER: &str = "commit ";

/// The stage manifests of the checkout at `root`, oldest first.
pub(crate) fn chronological_manifests(root: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let folder = manifests_folder();
    let mut found = stage_manifests(&root.join(&folder))?;
    let added = first_additions(root, &folder)?;
    found.sort_by_cached_key(|path| {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let commit = added
            .get(&format!("{folder}/{name}"))
            .copied()
            .unwrap_or(usize::MAX);
        (commit, name)
    });
    Ok(found)
}

/// Every `.tsv` in `folder` except the format sample, in name order.
fn stage_manifests(folder: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let entries =
        std::fs::read_dir(folder).map_err(|_| NotRun::TargetMissing(folder.to_path_buf()))?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "tsv")
                && path
                    .file_name()
                    .is_some_and(|name| name != EXAMPLE_MANIFEST)
        })
        .collect();
    found.sort();
    Ok(found)
}

/// The index, oldest first, of the commit that first added each file below `folder`, keyed by
/// repository path; empty before the first commit.
fn first_additions(root: &Path, folder: &str) -> Result<HashMap<String, usize>, NotRun> {
    if !has_head_commit(root)? {
        return Ok(HashMap::new());
    }
    if git(root, &["rev-parse", "--is-shallow-repository"])?.trim() == "true" {
        return Err(NotRun::ToolError {
            tool: "git log over the relocation manifests".to_string(),
            status: -1,
            stderr: "the checkout is a shallow clone, so the order the manifests entered the \
                     history cannot be read; fetch the full history"
                .to_string(),
        });
    }
    let listing = git(
        root,
        &[
            "-c",
            "core.quotePath=false",
            "log",
            "--topo-order",
            "--reverse",
            "--no-renames",
            "--diff-filter=A",
            "--format=format:commit %H",
            "--name-only",
            "--",
            folder,
        ],
    )?;
    Ok(parse_additions(&listing))
}

/// The first commit index of each file in a `git log --name-only` listing whose commits are
/// introduced by [`COMMIT_MARKER`] lines.
fn parse_additions(listing: &str) -> HashMap<String, usize> {
    let mut added = HashMap::new();
    let mut commit = 0usize;
    for line in listing.lines() {
        if line.starts_with(COMMIT_MARKER) {
            commit += 1;
        } else if !line.trim().is_empty() {
            added.entry(line.to_string()).or_insert(commit);
        }
    }
    added
}
