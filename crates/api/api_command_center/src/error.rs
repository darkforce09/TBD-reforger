//! Why a command center read outside a handler failed.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** the fleet overview read
//! ([`crate::services::fleet_overview::load_fleet_overview`]) reports a failed query as an
//! [`Error`]; the dashboard handler converts it into the handler error [`ApiError`], which every
//! other handler answers directly.
//! **Signals & state:** none; plain data.
//! **Invariants:** every variant is transparent, so its message and its causes read exactly as the
//! underlying error renders them; a database failure answers the [`ApiError`] a database failure
//! answers everywhere.

use api_foundation::error_handling::api_error::ApiError;

/// Why a command center read outside a handler failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The servers or their status rows could not be read.
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// The result of a fallible command center read a caller keeps apart from its own errors.
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
