//! Why a timestamp is not a canonical UTC RFC 3339 value.
//!
//! **Role:** the crate's one error type and its `Result` alias.
//! **Position:** returned by [`crate::validate_rfc3339_utc`]; a caller that stores timestamps
//! refuses the value and reports the message.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant names the offending field and value, and its message is fixed
//! per variant, so an operator reads the same text for the same rejection everywhere.

/// Why a timestamp fails the canonical-UTC rule, naming the field and the value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The value does not parse as an RFC 3339 date-time.
    #[error("{field} {value:?} is not an RFC 3339 date-time: {reason}")]
    NotRfc3339 {
        /// The name of the field that holds the value.
        field: String,
        /// The value as written.
        value: String,
        /// The parser's description of the failure.
        reason: String,
    },
    /// The value carries a non-zero offset.
    #[error("{field} {value:?} must be UTC (offset {offset}); write `Z` or `+00:00`")]
    NotUtc {
        /// The name of the field that holds the value.
        field: String,
        /// The value as written.
        value: String,
        /// The offset the value carries, as `±HH:MM:SS`.
        offset: String,
    },
    /// The value writes its zero offset other than as `Z` or `+00:00` (`-00:00`, lowercase `z`).
    #[error(
        "{field} {value:?} must write UTC as `Z` or `+00:00` \
         (`-00:00` means offset-unknown and lowercase `z` is non-canonical)"
    )]
    NonCanonicalUtcSpelling {
        /// The name of the field that holds the value.
        field: String,
        /// The value as written.
        value: String,
    },
    /// The value separates date and time with something other than an uppercase `T`.
    #[error("{field} {value:?} must separate date and time with an uppercase `T`")]
    LowercaseSeparator {
        /// The name of the field that holds the value.
        field: String,
        /// The value as written.
        value: String,
    },
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
