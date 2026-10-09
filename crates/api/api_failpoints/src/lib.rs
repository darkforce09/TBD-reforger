//! Fault injection for the API's test builds: named points on the commit and external-effect
//! paths where a test makes the path fail or pause, so the failure and race suites reach states a
//! live request cannot reach on demand.
//!
//! **Role:** the `fail_point!` macro a call site places, and, with the `failpoints` feature, the
//! `Failpoint` catalogue, the process-global registry of armed failpoints, the suite lock that
//! arms them, and the `Error` an armed failpoint returns.
//! **Position:** above `api_foundation`, whose `ApiError` the injected failure converts into.
//! The API domain crates expand the macro in their `async` transaction and effect paths; the
//! `failure_injection*` and `controlled_races*` suites under `crates/api/api_server/tests/` arm catalogue
//! entries through `lock_suite`, and this crate's own unit tests arm two entries that exist only
//! in its unit-test build.
//! **Signals & state:** with the feature on, one process-global registry (a `std::sync::Mutex`
//! over the armed entries) and one process-global suite lock (a `tokio::sync::Mutex`); with the
//! feature off, none.
//! **Invariants:** only `[dev-dependencies]` edges turn the feature on, so a deploy build
//! (`cargo build --release -p api_server --bin api-server`) expands every call site to nothing
//! and carries no registry, no catalogue name and no state; an unarmed failpoint is inert; a
//! failpoint is armed at most once at a time, only through a held suite lock, and disarms when its
//! `ArmGuard` drops, which also releases a pause it still holds.

#[cfg(feature = "failpoints")]
mod catalogue;
#[cfg(feature = "failpoints")]
mod error;
mod fail_point_macro;
#[cfg(feature = "failpoints")]
mod pause_handle;
pub mod prelude;
#[cfg(feature = "failpoints")]
mod registry;

#[cfg(feature = "failpoints")]
pub use catalogue::{CATALOGUE, Failpoint};
#[cfg(feature = "failpoints")]
pub use error::{Error, Result};
#[cfg(feature = "failpoints")]
pub use pause_handle::{PAUSE_REACH_BOUND, PauseHandle};
#[cfg(feature = "failpoints")]
pub use registry::{ArmGuard, FailAction, FailpointSuiteLock, lock_suite, reach};
