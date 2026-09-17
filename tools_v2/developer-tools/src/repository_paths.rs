//! Runtime checkout discovery for command inputs and repository fixtures.
use anyhow::{Context, Result, bail};
use std::path::PathBuf;

pub fn find_repo_root() -> Result<PathBuf> {
    find_from(std::env::current_dir().context("cwd")?)
}

fn find_from(mut current: PathBuf) -> Result<PathBuf> {
    loop {
        if current.join(".ai/tickets/ROOT").is_file()
            || current.join(".ai/tickets/registry.json").is_file()
        {
            return Ok(current);
        }
        if !current.pop() {
            bail!("could not find repo root (.ai/tickets/ROOT)");
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
