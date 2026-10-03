//! A `PATH` that searches one more folder first and always keeps the system tools reachable.
//!
//! **Role:** [`PathGuard`] prepends a folder to the process `PATH` for its lifetime, so
//! [`crate::which`] and every [`crate::Run`] spawned meanwhile find the programs in it first, and
//! restores the previous value on drop.
//! **Position:** used by `cargo xtask debug direct-join`, which searches `~/.local/bin` for its
//! probes, and by the xtask tests that put a stub program first on `PATH`. A test holds the
//! environment lock of the `tool_test_support` crate for the guard's whole life.
//! **Signals & state:** the process environment's `PATH`, written on creation and on drop.
//! **Invariants:** `PATH` is process-wide, and tests that spawn `git` and `grep` (the upstream-code
//! leak check among them) do not take the environment lock, so a new `PATH` always lists
//! `/usr/bin` and `/bin` (`prepended_path`); a guard restores exactly the value it found, an unset
//! `PATH` included. Prefer a tool-specific override variable (such as
//! `TBD_FETCH_VANILLA_API_CURL`) over a stub on `PATH`, and test the `PATH` construction through
//! the pure `prepended_path` rather than by rewriting the process `PATH`.

use std::ffi::OsString;
use std::path::Path;

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
    /// Never replaces PATH with a stub-only directory. Unit tests must hold the environment lock
    /// for the whole critical section that includes this guard. CLI one-shot entrypoints are
    /// single-threaded.
    pub fn prepend_dir(dir: &Path) -> Self {
        let previous = std::env::var_os("PATH");
        let old = std::env::var("PATH").unwrap_or_default();
        let next = prepended_path(&dir.display().to_string(), &old);
        // SAFETY: serialized in tests via the environment lock; a CLI run is single-threaded.
        #[allow(
            unsafe_code,
            reason = "std offers no safe call that writes a process variable"
        )]
        unsafe {
            std::env::set_var("PATH", next)
        };
        Self { previous }
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        // SAFETY: restoring the value we captured; same single-thread / lock contract as set.
        #[allow(
            unsafe_code,
            reason = "std offers no safe call that writes a process variable"
        )]
        unsafe {
            match self.previous.take() {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/search_path_tests.rs"]
mod tests;
