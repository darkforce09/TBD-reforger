//! Why a prefab catalogue or a world payload cannot be read.
//!
//! **Role:** the crate's one error type and its `Result` alias, gathering the two error families
//! the crate reports: [`WorldError`] for a world payload (gzip, JSON, archive, manifest, prefab id)
//! and [`BinaryError`] for the prefab archive itself; and [`InvalidPrefabId`], the catalogue row
//! the JSON narrowing refuses.
//! **Position:** converted into with `?` from every fallible call of the crate.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant wraps its family's error transparently: the message and the
//! source are the wrapped error's own.

use world_file_formats::archives::codec::BinaryError;

use crate::world_payload::WorldError;

/// Why a prefab catalogue or world payload is refused.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A world payload failed to decode.
    #[error(transparent)]
    Payload(#[from] WorldError),
    /// The prefab archive is malformed.
    #[error(transparent)]
    Archive(#[from] BinaryError),
}

/// A catalogue row whose numeric `prefabId` is not a whole number in `0..=u32::MAX`.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[error(
    "prefab row {row} has prefabId {value}, which is not a whole number in 0..=4294967295 — the \
     catalogue is addressed by a u32 prefab id"
)]
pub struct InvalidPrefabId {
    /// Index of the row in the catalogue's `prefabs` array.
    pub row: usize,

    /// The `prefabId` value the row carries.
    pub value: f64,
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
