//! A terrain's world object chunks and its terrain manifest.
//!
//! **Role:** decodes a world object chunk, from chunk JSON ([`world_chunk`]) or from a `TBDC`
//! container ([`chunk_container`]), into one column layout; names chunks with [`ChunkId`]
//! ([`chunk_id`]); and narrows the terrain manifest that says where every served map file is
//! ([`terrain_manifest`]).
//! **Position:** world formats category, tier 3, over `prefab_catalog` (the prefab map and render
//! classes a chunk row joins against), `world_file_formats` (the container header and object rows)
//! and `map_coordinates` (the chunk grid's identifier spelling). The map engine's world store,
//! scheduler and loaders and the developer tools' verification read it.
//! **Signals & state:** none; plain data types and pure functions.
//! **Invariants:** both decoders give bit-identical columns for the same chunk; a malformed buffer
//! is an error value, never a panic.

pub mod chunk_container;
pub mod chunk_id;
mod error;
pub mod prelude;
pub mod terrain_manifest;
pub mod world_chunk;

#[cfg(any(test, feature = "test_fixtures"))]
pub mod test_fixtures;

pub use chunk_id::ChunkId;
pub use error::{Error, Result};
