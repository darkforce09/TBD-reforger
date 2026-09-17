//! Process-global env helpers for xtask tests (and PATH-prepending CLI ports).
//!
//! `PATH` is process-wide. Replacing it with a stub-only directory races siblings that need
//! real `/usr/bin/grep` (notably [`crate::gate_crf_leak`]) under `cargo test --test-threads=N`.
//!
//! Rules:
//! 1. Prefer tool-specific env override seams (see `TBD_FETCH_VANILLA_API_CURL`) over PATH stubs.
//! 2. When PATH must change: prepend onto a PATH that still includes `/usr/bin:/bin`.
//! 3. Hold [`ENV_LOCK`] across every PATH mutate (and restore) in unit tests.

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

/// Sets `PATH` for the guard's lifetime; restores the previous value on drop.
pub struct PathGuard {
    previous: Option<OsString>,
}

impl PathGuard {
    /// Prepend `dir` onto `PATH`, always keeping `/usr/bin:/bin` reachable.
    ///
    /// Never replaces PATH with a stub-only directory. Unit tests must hold [`lock_env`] for the
    /// whole critical section that includes this guard. CLI one-shot entrypoints are single-threaded.
    pub fn prepend_dir(dir: &Path) -> Self {
        let previous = std::env::var_os("PATH");
        let dir = dir.display().to_string();
        let old = std::env::var("PATH").unwrap_or_default();
        let next = if old.is_empty() {
            format!("{dir}:/usr/bin:/bin")
        } else if path_has_system_bins(&old) {
            format!("{dir}:{old}")
        } else {
            format!("{dir}:/usr/bin:/bin:{old}")
        };
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
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn prepend_dir_keeps_usr_bin() {
        let _g = lock_env();
        let previous = std::env::var_os("PATH");
        // SAFETY: under ENV_LOCK; restored below.
        unsafe { std::env::set_var("PATH", "/tmp/only-stub-bin") };
        let stub = PathBuf::from("/tmp/t872-path-guard-stub");
        let _path = PathGuard::prepend_dir(&stub);
        let now = std::env::var("PATH").unwrap();
        assert!(now.starts_with("/tmp/t872-path-guard-stub:"));
        assert!(now.contains("/usr/bin"), "PATH must keep /usr/bin: {now}");
        assert!(now.contains("/bin"), "PATH must keep /bin: {now}");
        drop(_path);
        match previous {
            Some(p) => unsafe { std::env::set_var("PATH", p) },
            None => unsafe { std::env::remove_var("PATH") },
        }
    }
}
