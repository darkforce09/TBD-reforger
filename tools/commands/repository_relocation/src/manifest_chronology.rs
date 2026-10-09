//! The stage manifests in the order they entered the history.
//!
//! **Role:** lists every stage manifest of the manifests folder (every `.tsv` but the format
//! sample) and orders them chronologically, so `--verify` can follow an earlier manifest's scopes
//! through the moves of the manifests after it, and tell where those moves revived its retired
//! spellings ([`super::scope_history`]). A manifest keeps its
//! place when it moves: a rename is followed back to the commit that first added the file, so
//! moving the whole manifests folder leaves the order unchanged.
//!
//! **Position:** read by `cargo xtask refactor relocate --verify`
//! ([`super::relocation_modes::verify`]); the order comes from one `git log` over every `.tsv`
//! path of the history and one `git diff` of the index against `HEAD` for moves not committed yet.
//!
//! **Signals & state:** none; one directory listing and at most four git queries per call.
//!
//! **Invariants:** a manifest's place is the first commit of `HEAD`'s history that added it under
//! any path it has had: the log (`git log --topo-order --reverse --find-renames=100%
//! --diff-filter=AR`) is read oldest first, an addition starts a lineage and an exact rename
//! carries the lineage to the new path, and a staged exact rename against `HEAD` (a `git mv` not
//! committed yet) carries it the same way; only exact renames count, since a committed manifest is
//! never edited, so a new manifest that resembles a deleted one is never taken for it; manifests
//! whose lineages start in one commit sort by file name; manifests no commit added (untracked, or
//! added to the index but never committed under any path) come after every committed one, by file
//! name; a checkout without a commit orders every manifest by file name; a shallow clone, whose
//! history is cut, and a git query that fails are a [`NotRun`], never a guessed order.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use verification_core::NotRun;

use super::file_treatment::manifests_folder;
use super::relocation_modes::EXAMPLE_MANIFEST;
use super::repository_files::{git, has_head_commit};

/// The line `git log` prints before the files of each commit.
const COMMIT_MARKER: &str = "commit ";

/// The pathspec of every path a manifest can have had: a manifest is a `.tsv` file wherever its
/// folder lay, and rename detection pairs only the two sides of a move that the pathspec holds.
const MANIFEST_PATHSPEC: &str = "*.tsv";

/// The rename similarity git must find: an exact copy of the file's bytes.
const EXACT_RENAMES: &str = "--find-renames=100%";

/// The stage manifests of the checkout at `root`, oldest first.
pub(crate) fn chronological_manifests(root: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let folder = manifests_folder();
    let mut found = stage_manifests(&root.join(&folder))?;
    let added = first_additions(root)?;
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

/// The index, oldest first, of the commit that first added each `.tsv` file's lineage, keyed by
/// every repository path the lineage has had, the staged moves included; empty before the first
/// commit.
fn first_additions(root: &Path) -> Result<HashMap<String, usize>, NotRun> {
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
    let history = git(
        root,
        &[
            "-c",
            "core.quotePath=false",
            "log",
            "--topo-order",
            "--reverse",
            EXACT_RENAMES,
            "--diff-filter=AR",
            "--format=format:commit %H",
            "--name-status",
            "--",
            MANIFEST_PATHSPEC,
        ],
    )?;
    let staged = git(
        root,
        &[
            "-c",
            "core.quotePath=false",
            "diff",
            "--cached",
            EXACT_RENAMES,
            "--diff-filter=R",
            "--name-status",
            "HEAD",
            "--",
            MANIFEST_PATHSPEC,
        ],
    )?;
    let mut lineages = parse_lineages(&history);
    record_changes(&mut lineages, &staged, None);
    Ok(lineages)
}

/// The first commit index of each path's lineage in a `git log --name-status` listing whose
/// commits, oldest first, are introduced by [`COMMIT_MARKER`] lines.
fn parse_lineages(listing: &str) -> HashMap<String, usize> {
    let mut lineages = HashMap::new();
    let mut commit = 0usize;
    for line in listing.lines() {
        if line.starts_with(COMMIT_MARKER) {
            commit += 1;
        } else {
            record_changes(&mut lineages, line, Some(commit));
        }
    }
    lineages
}

/// Record `--name-status` lines: an `A` line starts the lineage of its path at `commit`; an
/// `R<score>` line gives its new path the lineage of its old one, or starts one at `commit` when
/// the old path has none. `commit` is `None` for staged changes, which start no lineage. A path
/// that already has a lineage keeps it.
fn record_changes(lineages: &mut HashMap<String, usize>, changes: &str, commit: Option<usize>) {
    for line in changes.lines() {
        let mut fields = line.split('\t');
        let status = fields.next().unwrap_or_default();
        let (origin, path) = match (status.chars().next(), fields.next(), fields.next()) {
            (Some('A'), Some(path), None) => (commit, path),
            (Some('R'), Some(old), Some(new)) => (lineages.get(old).copied().or(commit), new),
            _ => continue,
        };
        if let Some(origin) = origin {
            lineages.entry(path.to_string()).or_insert(origin);
        }
    }
}
