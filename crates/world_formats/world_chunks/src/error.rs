//! Why a world chunk cannot be read.
//!
//! **Role:** the crate's one error type and its `Result` alias, gathering the two error families
//! the chunk decoders report: [`BinaryError`] for a malformed container and [`ChunkBinError`] for
//! a container that is not the chunk requested.
//! **Position:** converted into with `?` from every fallible call of the crate.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant wraps its family's error transparently: the message and the
//! source are the wrapped error's own.

use world_file_formats::archives::codec::BinaryError;

use crate::chunk_container::ChunkBinError;

/// Why a world chunk buffer is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The `TBDC` container is malformed.
    #[error(transparent)]
    Binary(#[from] BinaryError),
    /// The container is well formed but is not the chunk that was requested.
    #[error(transparent)]
    ChunkContainer(#[from] ChunkBinError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
