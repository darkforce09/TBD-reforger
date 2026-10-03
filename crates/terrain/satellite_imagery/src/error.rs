//! Why a satellite container cannot be read.
//!
//! **Role:** the crate's one error type and its `Result` alias, wrapping the container reader's
//! [`TbdSatError`].
//! **Position:** converted into with `?` from every fallible call of the crate.
//! **Signals & state:** none; plain data.
//! **Invariants:** the variant wraps the reader's error transparently: the message and the source
//! are the wrapped error's own.

use crate::model::TbdSatError;

/// Why a satellite container is refused.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The header, the tile index or a tile's place in the file is malformed.
    #[error(transparent)]
    Container(#[from] TbdSatError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
