//! A settled fetch: the data, or a named reason there is none.
//!
//! **Role:** wraps the result of one request so that a render site cannot silently turn a refusal
//! into an empty list.
//! **Position:** the value pages hold in their resource signals and match on when rendering.
//! **Signals & state:** none of its own; it is the value a signal carries.
//! **Invariants:** the type deliberately offers no `Default`, no `unwrap_or_default`, no `ok` and
//! no `Deref`. Each of those is a one-token way to turn "the server refused to show you your data"
//! into "you have no data", which is the bug this type exists to prevent. Pages read it with a
//! `match`, which is total — it cannot be written without the failure arm.

use super::errors::{ApiErr, ApiFailure};

/// The outcome of a request that has finished: either the data, or why there is none.
///
/// Migrating a call site is one token: `.await.ok` becomes `.await.into`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fetched<T> {
    /// The backend answered and the body deserialised.
    Data(T),
    /// The request produced no data, for the named reason.
    Failed(ApiFailure),
}

impl<T> Fetched<T> {
    /// The data, when there is some.
    ///
    /// Borrowed on purpose: an owned `Option<T>` is one `unwrap_or_default` away from the
    /// empty-list bug.
    pub fn data(&self) -> Option<&T> {
        match self {
            Self::Data(t) => Some(t),
            Self::Failed(_) => None,
        }
    }

    /// True only when the session died — the banner predicate.
    ///
    /// Never true for a legitimately empty `Data`, which is the whole distinction this type draws.
    #[cfg(test)]
    pub fn is_session_expired(&self) -> bool {
        matches!(self, Self::Failed(f) if f.is_session_expired())
    }
}

/// Classify a finished request into data or a named failure.
impl<T> From<Result<T, ApiErr>> for Fetched<T> {
    fn from(r: Result<T, ApiErr>) -> Self {
        match r {
            Ok(t) => Self::Data(t),
            Err(e) => Self::Failed(e.into()),
        }
    }
}

#[cfg(test)]
#[path = "tests/fetched.rs"]
mod tests;
