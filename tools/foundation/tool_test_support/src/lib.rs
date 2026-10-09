//! The locks and the checkout root the tool crates' tests share.
//!
//! **Role:** [`lock_env`] serialises the tests that change process variables such as `PATH`;
//! [`CwdGuard`] and [`resolve_under_lock`] serialise the tests that change or read the working
//! directory; [`test_repo_root`] answers the checkout root a test reads fixtures from, under that
//! lock.
//! **Position:** tier 1 of `tools/foundation`, over `repository_root` (the root walk). Consumed
//! only from `[dev-dependencies]`; no production code links it. Its items are not
//! `cfg(test)`-gated, because a `cfg(test)` item is invisible to the crates that test with it.
//! **Signals & state:** two process-global mutexes, one for the environment and one for the working
//! directory; a guard holds its lock until it drops.
//! **Invariants:** every test in a process that changes the working directory holds the one
//! working-directory lock, and so does every reader that resolves the root from it; a guard
//! restores the previous working directory on drop, even when its test panicked; a poisoned lock
//! is taken over, never propagated.

mod environment_lock;
pub mod prelude;
mod test_checkout_root;
mod working_directory_lock;

pub use environment_lock::{ENV_LOCK, lock_env};
pub use test_checkout_root::test_repo_root;
pub use working_directory_lock::{CwdGuard, resolve_under_lock};
