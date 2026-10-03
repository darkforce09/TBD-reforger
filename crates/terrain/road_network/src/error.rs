//! Why a road network file cannot be read.
//!
//! **Role:** the crate's one error type and its `Result` alias, wrapping the [`BinaryError`] the
//! road network archive reports.
//! **Position:** converted into with `?` from the archive reader's error.
//! **Signals & state:** none; plain data.
//! **Invariants:** the variant wraps the format error transparently: the message and the source
//! are the wrapped error's own.

use world_file_formats::archives::codec::BinaryError;

/// Why a road network buffer is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The road network archive is misaligned, malformed, of another schema version, or names a
    /// road class code the class table cannot name.
    #[error(transparent)]
    Binary(#[from] BinaryError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
