//! Runtime checkout discovery for command inputs and repository fixtures.
//!
//! This walk is deliberately the second implementation of the one `ticket_engine::repository`
//! performs. Both crates are foundational: the structural rules forbid either from depending on
//! a workspace crate, so the alternative to two copies is an inverted dependency. The copies are
//! identical in behaviour — stop at the ticket-registry root marker, refuse when the walk reaches
//! the filesystem root — and each names the other.
use anyhow::{Context, Result, bail};
use std::path::PathBuf;

use crate::repository_layout::ROOT_MARKER;

pub fn find_repo_root() -> Result<PathBuf> {
    find_from(std::env::current_dir().context("cwd")?)
}

fn find_from(mut current: PathBuf) -> Result<PathBuf> {
    loop {
        if current.join(ROOT_MARKER).is_file() {
            return Ok(current);
        }
        if !current.pop() {
            bail!("could not find repo root ({ROOT_MARKER})");
        }
    }
}

#[cfg(test)]
pub(crate) fn test_repo_root() -> PathBuf {
    find_repo_root().expect("repo root")
}

#[cfg(test)]
#[path = "tests/repository_paths.rs"]
mod tests;
