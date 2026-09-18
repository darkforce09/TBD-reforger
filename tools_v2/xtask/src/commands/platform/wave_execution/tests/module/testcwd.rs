use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

static CWD_LOCK: Mutex<()> = Mutex::new(());

/// Run `f` with the cwd pinned — the READER half of this module's contract.
///
/// T-946. The module doc above already states the rule ("every test that moves it must hold
/// ONE lock"), but only the movers held it: a fixture helper resolving `find_repo_root()`
/// from the cwd took no lock at all, so it could still land inside a scratch tree mid-chdir.
/// Measured 2026-09-05 in the wave 248 gate, `test xtask+tbd-tools`:
/// `world_parity_world_column_clears_its_floor_when_the_dem_is_present` failed NotFound on a
/// committed fixture, 3/4 runs in the gate's cold target dir and never in isolation.
///
/// Resolve UNDER the lock and keep the absolute path: the answer stays valid after the lock
/// is released, so readers hold it for a path walk and nothing longer. Callers must not
/// already hold a [`CwdGuard`] — the mutex is not reentrant.
pub(crate) fn resolve_under_lock<T>(f: impl FnOnce() -> T) -> T {
    // Poisoning ignored for the guard's reason: a panicked sibling restored its cwd on drop.
    let _g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

/// Holds the lock and the previous cwd; restores the cwd on drop (lock released after).
pub(crate) struct CwdGuard {
    prev: PathBuf,
    _g: MutexGuard<'static, ()>,
}

impl CwdGuard {
    /// Take the lock, THEN resolve the target via `f`, then chdir. Resolution happens under
    /// the lock on purpose: `find_repo_root()` reads the cwd, so resolving before locking
    /// would race with whichever test currently owns it. `None` from `f` releases everything
    /// and returns `None` — the caller's skip path.
    pub(crate) fn enter_resolved(f: impl FnOnce() -> Option<PathBuf>) -> Option<CwdGuard> {
        // Poisoning is ignored on purpose: a panicked sibling must not cascade into every
        // later cwd test — the guard's Drop restored its cwd even on that panic.
        let g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = f()?;
        let prev = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&dir).expect("chdir");
        Some(CwdGuard { prev, _g: g })
    }

    pub(crate) fn enter(dir: &Path) -> CwdGuard {
        Self::enter_resolved(|| Some(dir.to_path_buf())).expect("enter with Some never skips")
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prev);
    }
}
