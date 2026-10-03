//! The failure an armed failpoint returns, and its conversions into the error types call sites
//! return.
//!
//! **Role:** [`Error::InjectedFailure`] names the failpoint that failed, and converts into
//! [`ApiError`] and [`sqlx::Error`] so the early return of the `fail_point!` macro type-checks in
//! handlers and in services alike.
//! **Position:** compiled only with the `failpoints` feature; [`crate::reach`] returns it and the
//! macro converts it at the call site.
//! **Signals & state:** none; plain values.
//! **Invariants:** the client of an injected failure sees `500` with `error = "internal error"`,
//! the body a real database failure answers, so a suite observes the production failure shape;
//! the [`ApiError`] conversion adds `details.failpoint` with the point's name, and the
//! [`sqlx::Error`] conversion is a `Protocol` error whose text names it, so a suite can tell an
//! injected failure from an accidental one.

use api_foundation::error_handling::api_error::ApiError;
use serde_json::json;

use crate::catalogue::Failpoint;

/// The failure an armed failpoint returns at its call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// An armed `Fail` or `FailOnce` failpoint failed the arrival.
    #[error("injected failure at failpoint {}", failpoint.name())]
    InjectedFailure {
        /// The point that failed.
        failpoint: Failpoint,
    },
}

impl Error {
    /// The point that failed.
    pub fn failpoint(&self) -> Failpoint {
        match self {
            Self::InjectedFailure { failpoint } => *failpoint,
        }
    }
}

/// The result of [`crate::reach`].
pub type Result<T> = std::result::Result<T, Error>;

/// A `500 internal error` carrying `details.failpoint`, logged the way a database failure is.
impl From<Error> for ApiError {
    fn from(failure: Error) -> Self {
        let name = failure.failpoint().name();
        tracing::error!(failpoint = name, "injected failure");
        Self::with_details(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "internal error",
            json!({ "failpoint": name }),
        )
    }
}

/// A `Protocol` error whose text names the failpoint; `ApiError` renders it as `500 internal
/// error` like every other database failure.
impl From<Error> for sqlx::Error {
    fn from(failure: Error) -> Self {
        sqlx::Error::Protocol(failure.to_string())
    }
}
