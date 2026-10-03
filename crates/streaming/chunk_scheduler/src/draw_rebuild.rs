//! The rebuild requests the chunk residency hands to the owner of the draw buffers.
//!
//! **Role:** names what a residency change leaves stale in the draw buffers composed over it, so
//! the scheduler never calls into the buffers: each mutation that changes what the buffers read
//! returns a [`DrawRebuild`], and the owner applies it at the same point.
//! **Position:** `chunk_scheduler`; returned by the viewport, ingest-frame and prefab-load
//! methods of [`crate::state::ChunkResidency`]; applied by
//! `chunk_draw_buffers::world_residency`.
//! **Signals & state:** none; plain values.
//! **Invariants:** a request names the cheapest rebuild that leaves the buffers equal to a full
//! recomposition; the owner applies every request before the next residency call.

use world_chunks::ChunkId;

/// What the draw buffers must rebuild after a residency change, and why.
#[must_use = "the draw buffers stay stale until their owner applies the rebuild"]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DrawRebuild {
    /// Nothing the draw buffers read changed.
    Nothing,

    /// The prefab tables were replaced: rebuild the prefab-to-glyph lookup.
    GlyphLookup,

    /// The pinned or resident chunks changed: recompose every draw buffer.
    AllBuffers,

    /// Only the viewport or the zoom moved, with no chunk pinned or the pin unchanged: refresh the
    /// draw set, and the glyph and strip buffers whose memo keys changed.
    DrawSetAndGlyphs,

    /// The pin is unchanged and the zoom moved from `previous_zoom` to `zoom`: recompose every
    /// draw buffer when the move crosses a building fill band, else refresh as
    /// [`DrawRebuild::DrawSetAndGlyphs`] does.
    ZoomUnderUnchangedPin {
        /// The deck zoom before the viewport call.
        previous_zoom: f64,

        /// The deck zoom of the viewport call.
        zoom: f64,
    },
}

/// The answer of [`crate::state::ChunkResidency::set_viewport`].
#[must_use = "the missing chunks must be fetched and the draw buffers rebuilt"]
#[derive(Clone, Debug, PartialEq)]
pub struct ViewportUpdate {
    /// The pinned chunks to fetch, already marked in flight, in chunk-math order.
    pub missing: Vec<ChunkId>,

    /// What the draw buffers must rebuild for the new viewport.
    pub rebuild: DrawRebuild,
}
