//! The identifier of one world object chunk.
//!
//! **Role:** declares [`ChunkId`], the `cx_cy` name of a chunk of the world object grid, as its
//! own type rather than a bare string.
//! **Position:** held by [`crate::world_chunk::WorldChunk`] and
//! [`crate::terrain_manifest::ChunkCell`], taken by the decoders of [`crate::world_chunk`] and
//! [`crate::chunk_container`]; the map engine's scheduler names chunks with it.
//! **Signals & state:** none; plain data.
//! **Invariants:** the identifier of grid cell `(cx, cy)` is spelled by the chunk grid's
//! `map_coordinates::chunk_math::chunk_id`, so every producer agrees; it serialises exactly as the
//! string it wraps.

use newtype_ids::string_id;

string_id! {
    /// A world object chunk's identifier, `cx_cy`, such as `18_0` or `-3_4`.
    pub struct ChunkId;
}

impl ChunkId {
    /// The identifier of the chunk at grid cell `(cx, cy)`.
    #[must_use]
    pub fn of_cell(cx: i64, cy: i64) -> Self {
        Self::new(map_coordinates::chunk_math::chunk_id(cx, cy))
    }
}

/// The empty identifier, which names no chunk: the placeholder a `WorldChunk::default()` holds
/// until its decoder or its builder sets the real one.
impl Default for ChunkId {
    fn default() -> Self {
        Self::new(String::new())
    }
}

#[cfg(test)]
#[path = "tests/chunk_id_tests.rs"]
mod tests;
