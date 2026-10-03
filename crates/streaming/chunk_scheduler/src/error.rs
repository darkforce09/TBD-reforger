//! Why a load or an ingest into the chunk residency is refused.
//!
//! **Role:** the crate's one error type and its `Result` alias: a world payload the payload
//! decoding refuses ([`WorldError`]: a manifest, prefab catalogue, chunk index or gzip chunk), and
//! a `TBDC` chunk container its reader refuses ([`ChunkBinError`]).
//! **Position:** returned by the loads and ingests of [`crate::chunk_ingest`].
//! **Signals & state:** none; plain data.
//! **Invariants:** both variants wrap their error transparently, so the message is the source's.

use prefab_catalog::world_payload::WorldError;
use world_chunks::chunk_container::ChunkBinError;

/// Why a load or an ingest is refused.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A payload failed to inflate or parse, was empty, named another terrain, or the manifest
    /// lacks the object-export paths.
    #[error(transparent)]
    Payload(#[from] WorldError),

    /// A `TBDC` chunk container is truncated, malformed or names another chunk.
    #[error(transparent)]
    ChunkContainer(#[from] ChunkBinError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
