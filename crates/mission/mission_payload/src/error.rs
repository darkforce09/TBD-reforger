//! Why a payload write fails.
//!
//! **Role:** the error of the crate's one fallible function, [`crate::version_body_to_writer`].
//! **Position:** returned through [`Result`]; the Mission Creator's document upload reads it.
//! **Signals & state:** none.
//! **Invariants:** an error's `Display` text is the underlying serializer's own message.

/// Why a payload write fails.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The version body could not be serialised into the caller's writer, or the writer refused
    /// the bytes.
    #[error(transparent)]
    VersionBodyWrite(#[from] serde_json::Error),
}

/// The result of a payload write.
pub type Result<T> = std::result::Result<T, Error>;
