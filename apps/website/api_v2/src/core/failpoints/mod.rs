//! Fault injection for test builds: named points on the commit and external-effect paths where a
//! test makes the path fail or pause, so the failure and race suites reach states a live request
//! cannot reach on demand.
//!
//! **Role:** the [`fail_point!`] macro a call site places before or after a commit or an external
//! effect, and, with the `failpoints` Cargo feature, the `Failpoint` catalogue, the process-global
//! registry of armed failpoints, the suite lock that arms them, and the `InjectedFailure` an armed
//! failpoint returns.
//! **Position:** `core`. Services and handlers of the domains expand the macro in their `async`
//! transaction and effect paths; the `failure_injection*` and `controlled_races*` suites under
//! `apps/website/api_v2/tests/` arm catalogue entries through `lock_suite`, and this module's own
//! unit tests arm two entries that exist only in the library's unit-test build.
//! **Signals & state:** with the feature on, one process-global registry (a `std::sync::Mutex`
//! over the armed entries) and one process-global suite lock (a `tokio::sync::Mutex`); with the
//! feature off, none.
//! **Invariants:** only the crate's self dev-dependency turns the feature on, so a deploy build
//! (`cargo build --release -p website-api --bin api`) expands every call site to nothing and
//! carries no registry, no catalogue name and no state; an unarmed failpoint is inert; a failpoint
//! is armed at most once at a time, only through a held suite lock, and disarms when its
//! `ArmGuard` drops, which also releases a pause it still holds.

/// Passes the named failpoint: inert unless a test armed it, and nothing at all in a build
/// without the `failpoints` feature.
///
/// The argument is a bare `Failpoint` variant name. Place it as a statement in an `async fn`
/// whose error type implements `From<InjectedFailure>` (`ApiError`, `sqlx::Error` and
/// `anyhow::Error` do): `fail_point!(SessionRotationBeforeCommit);`. An armed `Fail` or
/// `FailOnce` returns `Err(From::from(injected_failure))` from the enclosing function at that
/// point, which drops an open transaction and so rolls it back; an armed `Pause` holds the
/// enclosing future there until the test releases it.
#[cfg(feature = "failpoints")]
#[macro_export]
macro_rules! fail_point {
    ($failpoint:ident) => {
        if let ::core::result::Result::Err(injected_failure) =
            $crate::core::failpoints::reach($crate::core::failpoints::Failpoint::$failpoint).await
        {
            return ::core::result::Result::Err(::core::convert::From::from(injected_failure));
        }
    };
}

/// Passes the named failpoint: without the `failpoints` feature the call expands to nothing, so
/// a deploy build keeps no code, no name and no state for it.
#[cfg(not(feature = "failpoints"))]
#[macro_export]
macro_rules! fail_point {
    ($failpoint:ident) => {};
}

/// The macro under this module's path, so a call site imports it as
/// `use crate::core::failpoints::fail_point;`.
pub use crate::fail_point;

#[cfg(feature = "failpoints")]
mod catalogue;
#[cfg(feature = "failpoints")]
mod injected_failure;
#[cfg(feature = "failpoints")]
mod pause_handle;
#[cfg(feature = "failpoints")]
mod registry;

#[cfg(feature = "failpoints")]
pub use catalogue::{CATALOGUE, Failpoint};
#[cfg(feature = "failpoints")]
pub use injected_failure::InjectedFailure;
#[cfg(feature = "failpoints")]
pub use pause_handle::{PAUSE_REACH_BOUND, PauseHandle};
#[cfg(feature = "failpoints")]
pub use registry::{ArmGuard, FailAction, FailpointSuiteLock, lock_suite, reach};

#[cfg(all(test, feature = "failpoints"))]
#[path = "tests/fail_point_macro.rs"]
mod tests;
