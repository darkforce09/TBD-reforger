use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

pub fn find_repo_root() -> Result<PathBuf> {
    let mut cur = std::env::current_dir().context("cwd")?;
    loop {
        if cur.join(".ai/tickets/ROOT").is_file() || cur.join(".ai/tickets/registry.json").is_file()
        {
            return Ok(cur);
        }
        if !cur.pop() {
            bail!("could not find repo root (.ai/tickets/ROOT)");
        }
    }
}

/// The repo root of the checkout THIS BINARY WAS BUILT FROM, resolved at compile time from
/// `CARGO_MANIFEST_DIR` (`<root>/xtask`) — never from the cwd.
///
/// For test fixtures only. [`find_repo_root`] reads the process cwd, and the cwd is process-wide:
/// the wave tests `set_current_dir` into throwaway roots that carry their own `.ai/tickets/ROOT`
/// marker (under `wave::testcwd::CWD_LOCK`, which fixture readers do not hold), so any fixture
/// helper walking from the cwd at that instant resolved the throwaway root and failed with
/// NotFound. Measured 2026-09-05, wave 248 full gate (`test xtask+tbd-tools`), on
/// `map_world_los::tests::world_parity_world_column_clears_its_floor_when_the_dem_is_present`:
/// 3/4 red in the gate's cold target dir, never in isolation. A linked worktree has its own
/// `xtask/` so this resolves per worktree, exactly like the cwd walk did on a quiet process.
#[cfg(test)]
pub fn built_repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ lives directly under the repo root")
        .to_path_buf()
}

pub fn registry_path(root: &Path) -> PathBuf {
    root.join(".ai/tickets/registry.json")
}

pub fn gap_analysis_path(root: &Path) -> PathBuf {
    root.join("docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md")
}
