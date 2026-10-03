//! Why a worker pass an integration suite or the binary runs directly failed.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** the passes a caller runs outside their schedule
//! ([`crate::event_reservation_reevaluator::drain_due_reevaluations`],
//! [`crate::runtime_session_expiry::expire_runtime_sessions`]) report the failure of the domain
//! service they call as an [`Error`], which converts back into the handler error [`ApiError`].
//! **Signals & state:** none; plain data.
//! **Invariants:** an error renders exactly the message the service answered, so a worker's log
//! line reads the same as the service's refusal; converting it back yields that [`ApiError`]
//! unchanged.

use api_foundation::error_handling::api_error::ApiError;

/// Why a worker pass failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The domain service the pass calls refused or failed; it carries the status and message
    /// the service answered.
    #[error("{}", .0.message)]
    Service(ApiError),
}

/// The result of a worker pass a caller runs directly.
pub type Result<T> = std::result::Result<T, Error>;

/// A service failure becomes the pass's failure unchanged.
impl From<ApiError> for Error {
    fn from(failure: ApiError) -> Self {
        Error::Service(failure)
    }
}

/// The handler error the service answered.
impl From<Error> for ApiError {
    fn from(failure: Error) -> Self {
        match failure {
            Error::Service(source) => source,
        }
    }
}
