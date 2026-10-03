//! Test-only writers of the residency state the draw buffer tests set directly.
//!
//! **Role:** lets the residency tests place the viewport and zoom, and rewrite a resident chunk's
//! rows, without a fetch, so a single draw-buffer refresh can be checked against a reference.
//! **Position:** `chunk_scheduler`; compiled only in test builds and behind the `test_fixtures`
//! feature; called by the world residency tests of `chunk_draw_buffers`.
//! **Signals & state:** writes the residency's viewport, zoom, chunks and content epoch.
//! **Invariants:** none beyond the caller's; these writers bypass the pin and rebuild requests on
//! purpose, so production code never calls them.

use crate::state::ChunkResidency;
use map_coordinates::chunk_math::Bbox;
use world_chunks::ChunkId;
use world_chunks::world_chunk::WorldChunk;

impl ChunkResidency {
    /// Sets the deck zoom the draw buffers read, without a viewport pass.
    pub fn set_deck_zoom_for_test(&mut self, deck_zoom: f64) {
        self.deck_zoom = deck_zoom;
    }

    /// Sets the viewport the draw buffers read, without a viewport pass.
    pub fn set_last_viewport_for_test(&mut self, viewport: Bbox) {
        self.last_viewport = viewport;
    }

    /// The resident chunk `id`, writable, when it is resident.
    pub fn resident_chunk_mut_for_test(&mut self, id: &ChunkId) -> Option<&mut WorldChunk> {
        self.chunks.get_mut(id.as_str())
    }

    /// Bumps the content epoch, as an insert does, after a test rewrote a resident chunk.
    pub fn bump_content_epoch_for_test(&mut self) {
        self.content_epoch += 1;
    }
}
