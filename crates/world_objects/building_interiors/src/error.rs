//! Why a building compound cannot be assembled.
//!
//! **Role:** the crate's one error type and its `Result` alias, wrapping the
//! [`CompoundError`] the compound assembly reports.
//! **Position:** converted into with `?` from the compound assembly's error.
//! **Signals & state:** none; plain data.
//! **Invariants:** the variant wraps the assembly error transparently: the message and the source
//! are the wrapped error's own.

use crate::compound::assembly::CompoundError;

/// Why a building's interior model is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The instance records name BLAS sidecars the loader did not provide.
    #[error(transparent)]
    Compound(#[from] CompoundError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
