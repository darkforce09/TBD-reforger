//! The chunk scheduler: which world chunks the map keeps resident.
//!
//! **Role:** the chunk residency ([`state::ChunkResidency`]): the viewport pin, the in-flight
//! marks, the fetch-failure cap and LRU eviction ([`viewport`]), the manifest, prefab and chunk
//! ingest ([`chunk_ingest`]), the per-frame ingest budget ([`budget`]), picks and lookups
//! ([`queries`]), the object index ([`world_object_index`]) and the rebuild requests a residency
//! change hands to the draw buffers ([`draw_rebuild`]).
//! **Position:** streaming category, tier 4, over `world_chunks`, `prefab_catalog`,
//! `world_file_formats`, `spatial_indexes`, `map_draw_lanes` and `map_coordinates`; owned and
//! driven by `chunk_draw_buffers`' world residency, which the map engine's loaders hold.
//! **Signals & state:** the residency's pin, in-flight, LRU, index and statistics state, private
//! to this crate.
//! **Invariants:** nothing here names the draw buffers; every change the draw buffers must see
//! leaves as a [`draw_rebuild::DrawRebuild`]; a chunk crosses the public surface as a
//! `world_chunks::ChunkId`.

pub mod budget;
pub mod chunk_ingest;
mod draw_inputs;
pub mod draw_rebuild;
mod error;
pub mod prelude;
pub mod queries;
pub mod state;
pub mod viewport;
pub mod world_object_index;

#[cfg(any(test, feature = "test_fixtures"))]
mod test_hooks;

pub use error::{Error, Result};
