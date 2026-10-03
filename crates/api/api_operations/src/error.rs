//! Why an operations read outside a handler failed.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** the service reads the workers and the other domains call on their own pool (the
//! lifecycle sweep, the catalog and fire-mission reads) hand back sqlx's own [`sqlx::Error`]
//! untouched; the reservation planners report a broken invariant as a static message, and the
//! ballistics catalog upload and store report their refusals as their own coded enums. The handlers
//! answer the handler error [`ApiError`] directly. A caller that keeps a failure of this crate
//! apart from its other errors wraps it in [`Error`], which converts into [`ApiError`].
//! **Signals & state:** none; plain data.
//! **Invariants:** every variant is transparent, so its message and its causes read exactly as the
//! underlying error renders them; a database failure answers the [`ApiError`] a database failure
//! answers everywhere.

use api_foundation::error_handling::api_error::ApiError;

/// Why an operations read outside a handler failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The events, reservations, fire missions or catalog versions could not be read or written.
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
