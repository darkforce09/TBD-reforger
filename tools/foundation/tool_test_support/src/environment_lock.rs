//! The lock every test that changes a process variable holds.
//!
//! **Role:** [`ENV_LOCK`] and [`lock_env`] serialise the tests that write process variables,
//! `PATH` above all, against each other.
//! **Position:** taken by the tests of the `xtask` command groups that prepend a stub folder to
//! `PATH` (through `process_runner::PathGuard`) or set a variable for the length of one test.
//! **Signals & state:** one process-global mutex.
//! **Invariants:** the lock serialises writers only: tests that spawn `git`, `grep` and other
//! system tools do not take it, so every `PATH` a test sets (including the value a guard restores)
//! keeps `/usr/bin:/bin` reachable; a lock poisoned by a panicked test is taken over, since that
//! test's guards restored what they changed on drop.

use std::sync::{Mutex, MutexGuard};

/// Serialises every test that mutates a process variable (or runs under a temporary `PATH`
/// rewrite).
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Takes [`ENV_LOCK`] for the length of the returned guard, taking over a poisoned lock.
pub fn lock_env() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
