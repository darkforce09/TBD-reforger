//! Why a caller identity read or lock failed.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** the account authority read and the identity and account locks
//! ([`crate::account_authority`], [`crate::identity_ownership`]) run on their caller's business
//! transaction and hand back sqlx's own [`sqlx::Error`] untouched, so the caller's `?` classifies
//! a lock failure exactly like the business write beside it. The session and machine checks
//! ([`crate::session_authorization`], [`crate::machine_caller`]) answer the handler error
//! [`ApiError`] directly, since their refusals are HTTP statuses. A caller that keeps a read
//! failure apart from its other errors wraps it in [`Error::Database`], which converts into
//! [`ApiError`].
//! **Signals & state:** none; plain data.
//! **Invariants:** [`Error::Database`] is the transparent sqlx error, so its message and its
//! causes read exactly as sqlx renders them, and its [`ApiError`] is the one a database failure
//! answers everywhere.

use api_foundation::error_handling::api_error::ApiError;

/// Why a caller identity read or lock failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The account, session, identity or credential rows could not be read or locked.
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
