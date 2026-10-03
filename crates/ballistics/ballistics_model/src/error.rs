//! Why a ballistics model value is refused.
//!
//! **Role:** the crate's one error type and its `Result` alias, gathering the five error families
//! the model reports.
//! **Position:** converted into with `?` from every fallible call of the crate, so a caller that
//! decodes a catalog, looks up a firing and flies it returns one error type.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant wraps its family's error transparently: the message and the
//! source are the wrapped error's own.

use crate::angular_units::AngularUnitsError;
use crate::catalog::{CatalogDecodeError, CatalogLookupError};
use crate::flight_model::FlightError;
use crate::wind::WindError;

/// Why a catalog, a lookup, a flight, a wind or a sight convention is refused.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not a readable catalog.
    #[error(transparent)]
    CatalogDecode(#[from] CatalogDecodeError),
    /// A weapon or shell is not in the catalog, or the weapon does not fire the shell.
    #[error(transparent)]
    CatalogLookup(#[from] CatalogLookupError),
    /// A flight's inputs are invalid or it never descends through the target height.
    #[error(transparent)]
    Flight(#[from] FlightError),
    /// A wind's speed or direction is invalid.
    #[error(transparent)]
    Wind(#[from] WindError),
    /// A sight convention is not one the model knows.
    #[error(transparent)]
    AngularUnits(#[from] AngularUnitsError),
}

/// The result of a fallible call of this crate; the error defaults to [`Error`].
pub type Result<T, E = Error> = core::result::Result<T, E>;
