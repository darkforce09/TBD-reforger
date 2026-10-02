//! The failure an armed failpoint returns, and its conversions into the error types call sites
//! return.
//!
//! **Role:** [`InjectedFailure`] names the failpoint that failed, and converts into
//! [`ApiError`] and [`sqlx::Error`] so the early return of the `fail_point!` macro type-checks in
//! handlers and in services alike.
//! **Position:** `core::failpoints`, compiled only with the `failpoints` feature; the registry
//! creates it and the macro converts it at the call site.
//! **Signals & state:** none; plain values.
//! **Invariants:** the client of an injected failure sees `500` with `error = "internal error"`,
//! the body a real database failure answers, so a suite observes the production failure shape;
//! the [`ApiError`] conversion adds `details.failpoint` with the point's name, and the
//! [`sqlx::Error`] conversion is a `Protocol` error whose text names it, so a suite can tell an
//! injected failure from an accidental one.

use std::fmt;

use serde_json::json;

use super::catalogue::Failpoint;
use crate::core::error_handling::api_error::ApiError;

/// The failure an armed `Fail` or `FailOnce` failpoint returns at its call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InjectedFailure {
    /// The point that failed.
    pub failpoint: Failpoint,
}

impl fmt::Display for InjectedFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "injected failure at failpoint {}",
            self.failpoint.name()
        )
    }
}

impl std::error::Error for InjectedFailure {}

/// A `500 internal error` carrying `details.failpoint`, logged the way a database failure is.
impl From<InjectedFailure> for ApiError {
    fn from(failure: InjectedFailure) -> Self {
        tracing::error!(failpoint = failure.failpoint.name(), "injected failure");
        Self::with_details(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "internal error",
            json!({ "failpoint": failure.failpoint.name() }),
        )
    }
}

/// A `Protocol` error whose text names the failpoint; `ApiError` renders it as `500 internal
/// error` like every other database failure.
impl From<InjectedFailure> for sqlx::Error {
    fn from(failure: InjectedFailure) -> Self {
        sqlx::Error::Protocol(failure.to_string())
    }
}
