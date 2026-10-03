//! Why a validation registry fails its self-check.
//!
//! **Role:** the error of the crate's one fallible function, [`crate::Registry::self_check`].
//! **Position:** returned through [`Result`]; [`crate::Registry::assert_self_check`] panics with
//! its `Display` text, and a service calls either once at startup.
//! **Signals & state:** none.
//! **Invariants:** the `Display` text names every failing rule as `rule <id>: <reason>`, joined
//! with `; `, behind `validation registry self-check failed: `.

use crate::SelfCheckFailure;

/// Why a validation registry fails its self-check.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// One or more rules stayed silent on the trip fixture they declare they fire on, or their own
    /// `applies` gate excludes it.
    #[error("validation registry self-check failed: {}", joined(.0))]
    SelfCheckFailed(Vec<SelfCheckFailure>),
}

/// The result of a registry self-check.
pub type Result<T> = std::result::Result<T, Error>;

/// Every failure as `rule <id>: <reason>`, joined with `; `.
fn joined(failures: &[SelfCheckFailure]) -> String {
    failures
        .iter()
        .map(SelfCheckFailure::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}
