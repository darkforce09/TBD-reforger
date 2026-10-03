//! Why a missions read, contract check or registry import failed.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** the mission lookups ([`crate::services::mission_lookup`]) run on their caller's
//! pool and hand back sqlx's own [`sqlx::Error`] untouched; the schema validators
//! ([`crate::contract::schema_validators`]) report a schema that does not compile as a
//! [`ContractError`]; the registry import ([`crate::services::registry_import`]) reports an
//! envelope it refuses as an [`ImportError`]. The handlers answer the handler error [`ApiError`]
//! directly. A caller that keeps a failure of this crate apart from its other errors wraps it in
//! [`Error`], which converts into [`ApiError`].
//! **Signals & state:** none; plain data.
//! **Invariants:** every variant is transparent, so its message and its causes read exactly as
//! the underlying error renders them; a database failure answers the [`ApiError`] a database
//! failure answers everywhere, a refused envelope a bad request, and a schema that does not
//! compile an internal error.

use api_foundation::error_handling::api_error::ApiError;

use crate::contract::schema_validators::ContractError;
use crate::services::registry_import::ImportError;

/// Why a missions read, contract check or registry import failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The missions, versions, artifacts, reviews, deployments, factions or registries could not
    /// be read or written.
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    /// An embedded mission schema did not compile.
    #[error(transparent)]
    Contract(#[from] ContractError),
    /// A registry envelope was refused or could not be stored.
    #[error(transparent)]
    Import(#[from] ImportError),
}

/// The result of a fallible call of this crate that a caller keeps apart from its own errors.
pub type Result<T> = std::result::Result<T, Error>;

/// The handler error of the underlying failure: a database failure answers what every database
/// failure answers, a refused registry envelope is the caller's bad request, and a schema that
/// does not compile is the server's internal error.
impl From<Error> for ApiError {
    fn from(failure: Error) -> Self {
        match failure {
            Error::Database(source) | Error::Import(ImportError::Db(source)) => {
                ApiError::from(source)
            }
            Error::Contract(source) | Error::Import(ImportError::Contract(source)) => {
                ApiError::internal(source.to_string())
            }
            Error::Import(refused) => ApiError::bad_request(refused.to_string()),
        }
    }
}
