//! The `fail_point!` macro: a named point on a commit or external-effect path where a test makes
//! the path fail or pause.
//!
//! **Role:** the macro a call site places before or after a commit or an external effect; with the
//! `failpoints` feature it runs `reach` and returns the injected failure through the
//! enclosing function's error type, without it it expands to nothing.
//! **Position:** exported at the crate root (`api_failpoints::fail_point`); the services and
//! handlers of the API's domains expand it in their `async` transaction and effect paths.
//! **Signals & state:** none of its own; an armed point reads the registry in `reach`.
//! **Invariants:** the expansion names its items through `$crate::`, so it compiles in any crate
//! that depends on this one; without the feature a call site keeps no code, no name and no state.

/// Passes the named failpoint: inert unless a test armed it, and nothing at all in a build
/// without the `failpoints` feature.
///
/// The argument is a bare `Failpoint` variant name. Place it as a statement in an `async fn`
/// whose error type implements `From<api_failpoints::Error>` (`ApiError` and `sqlx::Error` do):
/// `fail_point!(SessionRotationBeforeCommit);`. An armed `Fail` or
/// `FailOnce` returns `Err(From::from(injected_failure))` from the enclosing function at that
/// point, which drops an open transaction and so rolls it back; an armed `Pause` holds the
/// enclosing future there until the test releases it.
#[cfg(feature = "failpoints")]
#[macro_export]
macro_rules! fail_point {
    ($failpoint:ident) => {
        if let ::core::result::Result::Err(injected_failure) =
            $crate::reach($crate::Failpoint::$failpoint).await
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

#[cfg(all(test, feature = "failpoints"))]
#[path = "tests/fail_point_macro.rs"]
mod tests;
