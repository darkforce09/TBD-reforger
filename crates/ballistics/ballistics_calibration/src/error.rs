//! Why a calibration bundle cannot be read.
//!
//! **Role:** the crate's one error type and its `Result` alias. A judged bundle never fails:
//! every mismatch is a [`crate::CalibrationFailure`] of its report; only decoding refuses.
//! **Position:** converted into with `?` from the crate's fallible calls, the bundle's and the
//! pinned catalog's decodes.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant wraps its family's error transparently: the message and the
//! source are the wrapped error's own.

use ballistics_model::catalog::CatalogDecodeError;

use crate::CalibrationDecodeError;

/// Why a calibration bundle or the catalog it pins does not decode.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not a readable calibration bundle.
    #[error(transparent)]
    BundleDecode(#[from] CalibrationDecodeError),
    /// The bytes are not a readable catalog.
    #[error(transparent)]
    CatalogDecode(#[from] CatalogDecodeError),
}

/// The result of a fallible call of this crate; the error defaults to [`Error`].
pub type Result<T, E = Error> = core::result::Result<T, E>;
