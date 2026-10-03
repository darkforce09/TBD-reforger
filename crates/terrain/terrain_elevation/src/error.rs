//! Why an elevation raster cannot be decoded.
//!
//! **Role:** the crate's one error type and its `Result` alias, gathering the two decoders'
//! errors: [`PngError`] for the 16-bit PNG and [`BinaryError`] for the raw `TBDE` grid.
//! **Position:** converted into with `?` from either decoder, so a caller that tries the raw grid
//! and falls back to the PNG returns one error type.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant wraps its decoder's error transparently: the message and the
//! source are the wrapped error's own.

use world_file_formats::archives::codec::BinaryError;

use crate::png::PngError;

/// Why an elevation raster is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The PNG is unreadable or not 16-bit grayscale.
    #[error(transparent)]
    Png(#[from] PngError),
    /// The raw `TBDE` grid is truncated, too long, or declares dimensions the file cannot hold.
    #[error(transparent)]
    RawGrid(#[from] BinaryError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
