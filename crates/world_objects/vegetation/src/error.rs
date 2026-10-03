//! Why a forest regions file cannot be read or written.
//!
//! **Role:** the crate's one error type and its `Result` alias, wrapping the [`BinaryError`] the
//! forest regions archive reports.
//! **Position:** converted into with `?` from the archive reader's and writer's error.
//! **Signals & state:** none; plain data.
//! **Invariants:** the variant wraps the format error transparently: the message and the source
//! are the wrapped error's own.

use world_file_formats::archives::codec::BinaryError;

/// Why a forest regions buffer or region is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The forest regions archive is misaligned, malformed or of another schema version, or a
    /// region cannot be written to it.
    #[error(transparent)]
    Binary(#[from] BinaryError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
