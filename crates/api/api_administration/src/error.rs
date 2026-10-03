//! Why an administration read or write failed.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** the audit publication pass
//! ([`crate::services::audit_publication::publish_audit_batch`]) runs on its caller's pool and
//! hands back sqlx's own [`sqlx::Error`] untouched, so the publication worker and the integration
//! suites classify a failed pass exactly like their other reads. The handlers answer the handler
//! error [`ApiError`] directly. A caller that keeps a failure of this crate apart from its other
//! errors wraps it in [`Error::Database`], which converts into [`ApiError`].
//! **Signals & state:** none; plain data.
//! **Invariants:** [`Error::Database`] is the transparent sqlx error, so its message and its
//! causes read exactly as sqlx renders them, and its [`ApiError`] is the one a database failure
//! answers everywhere.

use api_foundation::error_handling::api_error::ApiError;

/// Why an administration read or write failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The roster, moderation or audit publication rows could not be read or written.
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
