//! Why a match telemetry body fails a shape check before its refusal is built.
//!
//! **Role:** the crate's error type and its `Result` alias: the shape failures the ingest
//! decoders share ([`crate::models::match_registration::source_match_id`],
//! [`crate::models::telemetry_refusal::check_object_keys`]).
//! **Position:** each wire decoder in [`crate::models`] turns an [`Error`] into its own coded
//! refusal (`INVALID_MATCH_REGISTRATION`, `INVALID_MATCH_RESULTS`, `INVALID_EVENT`) with the
//! message and the key the error names; the handlers and services answer the handler error
//! [`ApiError`] directly. A caller holding an [`Error`] without a refusal code of its own converts
//! it into a plain 400.
//! **Signals & state:** none; plain data.
//! **Invariants:** each message is the exact text a game runtime reads in the refusal, and
//! [`Error::offending_key`] is the exact `details.field` the refusal carries.

use api_foundation::error_handling::api_error::ApiError;

/// Why a match telemetry body fails a shape check.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A source match id that is blank or longer than 128 bytes once trimmed.
    #[error("source_match_id must contain 1 to 128 bytes")]
    SourceMatchIdLength,
    /// A request body that is not a JSON object.
    #[error("the body must be a JSON object")]
    BodyNotAnObject,
    /// A top-level key outside the keys the request allows.
    #[error("unknown field `{0}`")]
    UnknownField(String),
    /// A key the request requires that the body leaves out.
    #[error("missing field `{0}`")]
    MissingField(String),
}

impl Error {
    /// The key the refusal names as its `details.field`: the empty key for a body that is not an
    /// object, since the fault is the body itself.
    pub fn offending_key(&self) -> &str {
        match self {
            Error::SourceMatchIdLength => "source_match_id",
            Error::BodyNotAnObject => "",
            Error::UnknownField(key) | Error::MissingField(key) => key,
        }
    }
}

/// The result of a match telemetry shape check.
pub type Result<T> = std::result::Result<T, Error>;

/// A plain 400 carrying the shape failure's message.
impl From<Error> for ApiError {
    fn from(failure: Error) -> Self {
        ApiError::bad_request(failure.to_string())
    }
}

#[cfg(test)]
#[path = "tests/error.rs"]
mod tests;
