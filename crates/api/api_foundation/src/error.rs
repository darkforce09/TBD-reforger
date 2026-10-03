//! Why a wire value of this crate cannot be read.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** the date reader of [`crate::wire_format::rfc3339_utc_date`] refuses a string
//! with an [`Error`], which the module's `deserialize` hands to serde as its message.
//! **Signals & state:** none; plain data.
//! **Invariants:** an [`Error`] displays exactly as the parser error it carries, so a refused date
//! reads the same to a client whichever layer reports it.

/// Why a wire value of this crate cannot be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A date is neither an RFC 3339 timestamp nor a bare `YYYY-MM-DD` date.
    #[error(transparent)]
    MalformedDate(#[from] chrono::ParseError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
