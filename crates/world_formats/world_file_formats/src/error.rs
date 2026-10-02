//! Why a world file cannot be read or written.
//!
//! **Role:** the crate's one error type and its `Result` alias, gathering the two error families
//! the formats report: [`BinaryError`] for the archives, containers and object rows, and
//! [`TbddError`] for the density grids.
//! **Position:** converted into with `?` from every fallible call of the crate, so a caller that
//! reads several formats in one function returns one error type.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant wraps its family's error transparently: the message and the
//! source are the wrapped error's own.

use crate::archives::codec::BinaryError;
use crate::density::tbdd::TbddError;

/// Why a world file buffer is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// An rkyv archive, a fixed-header container or a run of object rows is malformed.
    #[error(transparent)]
    Binary(#[from] BinaryError),
    /// A vegetation density grid is malformed.
    #[error(transparent)]
    Density(#[from] TbddError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
