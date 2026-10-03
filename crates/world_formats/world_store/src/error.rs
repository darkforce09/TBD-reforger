//! Why a world payload cannot be loaded into the store.
//!
//! **Role:** the crate's one error type and its `Result` alias: a payload the world payload
//! decoding refuses ([`WorldError`]), and a road network archive its reader refuses
//! ([`BinaryError`]).
//! **Position:** returned by every load of [`crate::store::WorldStore`].
//! **Signals & state:** none; plain data.
//! **Invariants:** the payload variant wraps its error transparently; the road archive variant
//! keeps the archive's error as its source.

use prefab_catalog::world_payload::WorldError;
use world_file_formats::archives::codec::BinaryError;

/// Why a world payload is refused.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A payload failed to inflate or parse, was empty, or the manifest lacks the object-export
    /// paths.
    #[error(transparent)]
    Payload(#[from] WorldError),

    /// The road network archive is malformed, of another schema version, or names a road class
    /// code the class table cannot name.
    #[error("world: binary archive failed to load: {0}")]
    RoadArchive(#[from] BinaryError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
