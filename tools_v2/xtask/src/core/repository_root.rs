use anyhow::Result;
use std::path::PathBuf;

pub fn find_repo_root() -> Result<PathBuf> {
    ticket_engine::repository::find_repo_root()
}

/// The repo root for TEST FIXTURES: the cwd walk, resolved under the cwd lock.
///
/// T-946, and it replaced a `CARGO_MANIFEST_DIR` constant that looked simpler and was wrong. The
/// factory builds every worktree into ONE shared target dir, so the binary a slice worktree runs
/// may have been compiled from a SIBLING worktree, and a compile-time root then points at that
/// sibling's checkout — whose LFS payloads are pointer files. Measured 2026-09-05: five
/// `map_world_los` pins failed with
/// `parse …/worktrees/T-943/packages/map-assets/…/t_picea_abies_0_canopy.bvh: bad magic
/// [118, 101, 114, 115]` — "vers", the first bytes of `version https://git-lfs…`. That is the
/// T-742 cross-worktree false-binary class arriving through a constant instead of a binary.
///
/// So the answer must come from the cwd (which worktree am I actually running in?) while the
/// RACE that motivated all of this is closed by taking [`crate::commands::platform::wave_execution::testcwd`]'s lock — the one
/// the chdir-ing tests already hold. See that module for the measured failure.
///
/// Callers must not already hold a `CwdGuard`; the mutex is not reentrant.
#[cfg(test)]
pub fn test_repo_root() -> PathBuf {
    crate::commands::platform::wave_execution::testcwd::resolve_under_lock(|| {
        find_repo_root().expect("repo root")
    })
}

#[cfg(test)]
#[path = "../tests/repository_root_tests.rs"]
mod tests;
