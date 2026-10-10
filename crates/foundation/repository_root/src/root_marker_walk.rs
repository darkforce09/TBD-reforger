//! The walk that finds a checkout root.
//!
//! **Role:** [`find_repository_root`] walks up from the working directory and
//! [`find_repository_root_from`] from a given folder, each to the first folder holding
//! [`ROOT_MARKER`]; [`is_repository_root`] confirms a folder a caller already holds.
//! **Position:** the only checkout-root finder of the workspace; every tool command, tool test,
//! API test and frontend test that needs the root calls it, and joins its own relative locations
//! onto the answer.
//! **Signals & state:** none; reads the filesystem only.
//! **Invariants:** the walk stops at the nearest marker, so a worktree nested under another
//! checkout resolves to itself; only a file named by the marker counts, never a folder; reaching
//! the filesystem root is [`crate::Error::RootMarkerNotFound`].

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// The file whose presence marks a checkout root, relative to that root. It is a tracked file of
/// the agent folder `.ai/`, so every clone, CI checkout and slice worktree carries it, and only a
/// test that plants it on purpose makes a throwaway folder a root.
pub const ROOT_MARKER: &str = ".ai/ROOT";

/// The checkout root above the working directory.
///
/// Answering from the working directory rather than from a compile-time folder is what makes a
/// command run inside a slice worktree read that worktree's files: worktrees share one build
/// folder, so the binary may have been compiled from a sibling checkout.
pub fn find_repository_root() -> Result<PathBuf> {
    let start = std::env::current_dir().map_err(Error::CurrentDirectory)?;
    find_repository_root_from(&start)
}

/// The checkout root at or above `start`: the nearest folder that holds [`ROOT_MARKER`].
pub fn find_repository_root_from(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if is_repository_root(&current) {
            return Ok(current);
        }
        if !current.pop() {
            return Err(Error::RootMarkerNotFound {
                start: start.to_path_buf(),
            });
        }
    }
}

/// `true` when `candidate` is a checkout root, the probe for a caller that already holds a folder
/// and only needs to confirm it.
pub fn is_repository_root(candidate: &Path) -> bool {
    candidate.join(ROOT_MARKER).is_file()
}

#[cfg(test)]
#[path = "tests/root_marker_walk.rs"]
mod tests;
