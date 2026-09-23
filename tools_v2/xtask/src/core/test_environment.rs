//! Process-global env helpers for xtask tests (and PATH-prepending CLI ports).
//!
//! `PATH` is process-wide. Replacing it with a stub-only directory races siblings that need
//! real `/usr/bin/grep` (notably [`crate::verifications::licensing::upstream_code_leaks`]) under `cargo test --test-threads=N`.
//!
//! `ENV_LOCK` serialises PATH writers against each other only: tests that spawn `git`, `grep`,
//! and other system tools do not take it, so every PATH value a test sets (including the value a
//! guard restores) must keep `/usr/bin:/bin` reachable.
//!
//! Rules:
//! 1. Prefer tool-specific env override seams (see `TBD_FETCH_VANILLA_API_CURL`) over PATH stubs.
//! 2. When PATH must change: prepend onto a PATH that still includes `/usr/bin:/bin`.
//! 3. Hold `ENV_LOCK` across every PATH mutate (and restore) in unit tests.
//! 4. Test PATH construction through the pure `prepended_path` instead of rewriting process PATH.

use std::ffi::OsString;
use std::path::Path;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};

/// Serialise every test that mutates process `PATH` (or runs under a temporary PATH rewrite).
#[cfg(test)]
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
pub fn lock_env() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn path_has_system_bins(path: &str) -> bool {
    path.split(':').any(|p| p == "/usr/bin" || p == "/bin")
}

/// Builds the `PATH` value that searches `dir` first while keeping `/usr/bin` and `/bin` reachable.
///
/// - An empty `old` yields `<dir>:/usr/bin:/bin`.
/// - An `old` that already lists `/usr/bin` or `/bin` yields `<dir>:<old>`.
/// - Any other `old` (for example a stub-only directory) yields `<dir>:/usr/bin:/bin:<old>`, so
///   system tools such as `git` and `grep` still resolve.
fn prepended_path(dir: &str, old: &str) -> String {
    if old.is_empty() {
        format!("{dir}:/usr/bin:/bin")
    } else if path_has_system_bins(old) {
        format!("{dir}:{old}")
    } else {
        format!("{dir}:/usr/bin:/bin:{old}")
    }
}

/// Sets `PATH` for the guard's lifetime; restores the previous value on drop.
pub struct PathGuard {
    previous: Option<OsString>,
}

impl PathGuard {
    /// Prepend `dir` onto `PATH`, always keeping `/usr/bin:/bin` reachable (see `prepended_path`).
    ///
    /// Never replaces PATH with a stub-only directory. Unit tests must hold `lock_env` for the
    /// whole critical section that includes this guard. CLI one-shot entrypoints are single-threaded.
    pub fn prepend_dir(dir: &Path) -> Self {
        let previous = std::env::var_os("PATH");
        let old = std::env::var("PATH").unwrap_or_default();
        let next = prepended_path(&dir.display().to_string(), &old);
        // SAFETY: serialized in tests via ENV_LOCK; CLI is single-threaded.
        unsafe { std::env::set_var("PATH", next) };
        Self { previous }
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        // SAFETY: restoring the value we captured; same single-thread / lock contract as set.
        unsafe {
            match self.previous.take() {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/test_environment/tests.rs"]
mod tests;
