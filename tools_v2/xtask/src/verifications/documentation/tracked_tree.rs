//! The tracked files of a checkout and the folders they imply.
//!
//! **Role:** runs `git ls-files -z` at the checkout root and turns the listing into the set of
//! tracked files plus, for every folder that holds one, its direct tracked children.
//!
//! **Position:** the first step of every documentation gate ([`super::prepare`]); the gates and
//! [`super::gate_scope`] read the resulting [`TrackedTree`] and never walk the disk for structure.
//!
//! **Signals & state:** none; a tree is built once per run and read-only afterwards.
//!
//! **Invariants:** a folder exists in the tree exactly when at least one tracked file sits at or
//! below it, so untracked and ignored files never appear; git missing, killed, timed out or
//! exiting non-zero is a [`NotRun`] cause, never an empty tree that would read as a clean pass.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use verification_core::NotRun;
use verification_core::proc::{Output, Run};

/// The program that lists the tracked files.
const GIT: &str = "git";

/// `git ls-files -z`: every path in the index, NUL-separated and never quoted.
const LISTING_ARGUMENTS: [&str; 2] = ["ls-files", "-z"];

/// A listing that takes longer than this is killed and reported as a timeout: the whole index of
/// this repository lists in well under a second, so a stall is a broken git, not a large tree.
const LISTING_DEADLINE: Duration = Duration::from_secs(120);

/// The tracked children directly inside one folder.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct FolderChildren {
    /// Names of the tracked files directly inside the folder.
    pub(super) files: BTreeSet<String>,
    /// Names of the folders directly inside the folder that hold a tracked file.
    pub(super) folders: BTreeSet<String>,
}

/// The tracked files of a checkout and every folder that holds one.
#[derive(Debug, Default)]
pub(super) struct TrackedTree {
    /// Every tracked file, as a `/`-separated repository-relative path.
    files: BTreeSet<String>,
    /// Every folder that holds a tracked file, keyed by its repository-relative path; the
    /// repository root is the empty path.
    folders: BTreeMap<String, FolderChildren>,
}

impl TrackedTree {
    /// List the tracked files at `repo_root` with `git ls-files -z`.
    pub(super) fn load(repo_root: &Path) -> Result<TrackedTree, NotRun> {
        read_listing(
            Run::new(GIT)
                .args(LISTING_ARGUMENTS)
                .cwd(repo_root)
                .timeout(LISTING_DEADLINE),
        )
    }

    /// Build the tree from a NUL-separated listing. Empty entries are skipped.
    pub(super) fn from_listing(listing: &str) -> TrackedTree {
        let mut tree = TrackedTree::default();
        for path in listing.split('\0').filter(|path| !path.is_empty()) {
            tree.insert(path);
        }
        tree
    }

    /// Record one tracked file and every folder above it.
    fn insert(&mut self, path: &str) {
        self.files.insert(path.to_string());
        let (mut folder, name) = split_parent(path);
        self.folders
            .entry(folder.to_string())
            .or_default()
            .files
            .insert(name.to_string());
        while !folder.is_empty() {
            let (parent, name) = split_parent(folder);
            let children = self.folders.entry(parent.to_string()).or_default();
            if !children.folders.insert(name.to_string()) {
                break;
            }
            folder = parent;
        }
    }

    /// Every tracked file, in path order.
    pub(super) fn files(&self) -> impl Iterator<Item = &str> {
        self.files.iter().map(String::as_str)
    }

    /// Every folder that holds a tracked file, in path order, without the repository root.
    pub(super) fn folders(&self) -> impl Iterator<Item = &str> {
        self.folders
            .keys()
            .map(String::as_str)
            .filter(|folder| !folder.is_empty())
    }

    /// How many files are tracked.
    pub(super) fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Whether `path` is a tracked file.
    pub(super) fn is_file(&self, path: &str) -> bool {
        self.files.contains(path)
    }

    /// Whether `path` is a folder that holds a tracked file; the empty path is the repository
    /// root, which holds one whenever anything is tracked.
    pub(super) fn is_folder(&self, path: &str) -> bool {
        self.folders.contains_key(path)
    }

    /// The tracked children directly inside `folder`, or `None` when it holds no tracked file.
    pub(super) fn children(&self, folder: &str) -> Option<&FolderChildren> {
        self.folders.get(folder)
    }
}

/// Run a listing command and map its outcome onto a tree or onto the reason there is none.
pub(super) fn read_listing(command: Run) -> Result<TrackedTree, NotRun> {
    from_listing_output(command.output()?)
}

/// A finished listing's tree, or [`NotRun::ToolError`] when the listing exited non-zero: a
/// failed listing lists nothing, and nothing must never read as a clean tree.
pub(super) fn from_listing_output(output: Output) -> Result<TrackedTree, NotRun> {
    if output.code != 0 {
        return Err(NotRun::ToolError {
            tool: format!("{GIT} {}", LISTING_ARGUMENTS.join(" ")),
            status: output.code,
            stderr: output.stderr.trim().to_string(),
        });
    }
    Ok(TrackedTree::from_listing(&output.stdout))
}

/// `(folder, name)` for a repository-relative path; a top-level name sits in the root, `""`.
fn split_parent(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

#[cfg(test)]
#[path = "tests/tracked_tree.rs"]
mod tests;
