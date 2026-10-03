//! Why a spatial index cannot be read.
//!
//! **Role:** the crate's one error type and its `Result` alias; today it gathers the one fallible
//! family, the BVH sidecar parse ([`BvhParseError`]).
//! **Position:** converted into with `?` from every fallible call of the crate.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant wraps its family's error transparently: the message and the
//! source are the wrapped error's own.

use crate::bounding_volume_hierarchy::sidecar_parse_error::BvhParseError;

/// Why a spatial index buffer is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A BVH sidecar file is malformed.
    #[error(transparent)]
    Sidecar(#[from] BvhParseError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
