use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

pub fn find_repo_root() -> Result<PathBuf> {
    let mut cur = std::env::current_dir().context("cwd")?;
    loop {
        if cur.join(".ai/tickets/registry.json").is_file() {
            return Ok(cur);
        }
        if !cur.pop() {
            bail!("could not find repo root (.ai/tickets/registry.json)");
        }
    }
}

pub fn registry_path(root: &Path) -> PathBuf {
    root.join(".ai/tickets/registry.json")
}

pub fn gap_analysis_path(root: &Path) -> PathBuf {
    root.join("docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md")
}
