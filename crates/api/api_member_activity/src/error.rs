//! Why a member activity read or write failed.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** the leaderboard refresh ([`crate::leaderboard_view`]) runs on its caller's pool
//! or business transaction and hands back sqlx's own [`sqlx::Error`] untouched, so the caller's
//! `?` classifies a refresh failure exactly like the business write beside it. The statistics,
//! attribution and queue writers ([`crate::user_stats`], [`crate::participation_attribution`],
//! [`crate::reevaluation_queue`]) answer the handler error [`ApiError`] their callers return. A
//! caller that keeps a failure of this crate apart from its other errors wraps it in
//! [`Error::Database`], which converts into [`ApiError`].
//! **Signals & state:** none; plain data.
//! **Invariants:** [`Error::Database`] is the transparent sqlx error, so its message and its
//! causes read exactly as sqlx renders them, and its [`ApiError`] is the one a database failure
//! answers everywhere.

use api_foundation::error_handling::api_error::ApiError;

/// Why a member activity read or write failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The statistics, the leaderboard view, the attendance rows or the queue could not be read
    /// or written.
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// The result of a fallible call of this crate that a caller keeps apart from its own errors.
pub type Result<T> = std::result::Result<T, Error>;

/// The handler error of the underlying failure: a database failure answers what every database
/// failure answers.
impl From<Error> for ApiError {
    fn from(failure: Error) -> Self {
        match failure {
            Error::Database(source) => ApiError::from(source),
        }
    }
}
