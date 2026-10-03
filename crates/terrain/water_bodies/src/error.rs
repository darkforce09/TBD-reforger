//! Why a water file cannot be read.
//!
//! **Role:** the crate's one error type and its `Result` alias, wrapping the [`BinaryError`] the
//! bathymetry container, the water mask and the inland water archive report.
//! **Position:** converted into with `?` from every fallible call of the crate.
//! **Signals & state:** none; plain data.
//! **Invariants:** the variant wraps the format error transparently: the message and the source
//! are the wrapped error's own.

use world_file_formats::archives::codec::BinaryError;

/// Why a water file buffer is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The bathymetry container, the world rectangle or the inland water archive is malformed.
    #[error(transparent)]
    Binary(#[from] BinaryError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
