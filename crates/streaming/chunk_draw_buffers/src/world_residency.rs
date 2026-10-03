//! The world residency: the chunk residency and the draw buffers composed over it, as one owner.
//!
//! **Role:** owns a [`ChunkResidency`] and the draw buffers composed from it, applies every
//! rebuild request a residency change returns at the point the change happens, and keeps the
//! public method names the world loader, the occluder loader and the debug world line-of-sight
//! bench call.
//! **Position:** `chunk_draw_buffers`; the type the loaders hold; the residency calls that request
//! no rebuild pass through in `chunk_residency_delegates.rs`, the draw getters live in
//! `revision.rs`, the toggles in `toggles.rs` and the statistics in `residency_statistics.rs`.
//! **Signals & state:** the two owned halves; nothing else.
//! **Invariants:** the draw buffers are never stale after a public call returns: every
//! [`DrawRebuild`] is applied before the call that produced it returns.

use crate::draw_buffers::DrawBuffers;
use crate::error::Result;
use chunk_scheduler::draw_rebuild::DrawRebuild;
use chunk_scheduler::state::ChunkResidency;
use world_chunks::ChunkId;

/// Multi-chunk residency + LRU + world spatial index + building/glyph GPU-buffer composer.
#[derive(Default)]
pub struct WorldResidency {
    /// The chunk residency: pin, in-flight marks, LRU, object index, ingest statistics.
    pub(super) chunk_residency: ChunkResidency,

    /// The draw state composed from the chunk residency.
    pub(super) draw_buffers: DrawBuffers,
}

impl WorldResidency {
    /// An empty residency with empty draw buffers and the default layer toggles.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl WorldResidency {
    /// Brings the draw buffers up to date with the residency change that returned `rebuild`.
    pub(super) fn apply_draw_rebuild(&mut self, rebuild: DrawRebuild) {
        let residency = &self.chunk_residency;
        let buffers = &mut self.draw_buffers;
        match rebuild {
            DrawRebuild::Nothing => {}
            DrawRebuild::GlyphLookup => buffers.rebuild_glyph_lookup_from_prefabs(residency),
            DrawRebuild::AllBuffers => buffers.rebuild_buffers(residency),
            DrawRebuild::DrawSetAndGlyphs => buffers.refresh_draw_set_and_glyphs(residency),
            DrawRebuild::ZoomUnderUnchangedPin {
                previous_zoom,
                zoom,
            } => {
                if DrawBuffers::fill_band_changed(
                    residency.min_importance_zoom(),
                    previous_zoom,
                    zoom,
                ) {
                    buffers.rebuild_buffers(residency);
                } else {
                    buffers.refresh_draw_set_and_glyphs(residency);
                }
            }
        }
    }
}

impl WorldResidency {
    /// `runViewport(bbox, deckZoom)` — pin the viewport chunk set, re-touch resident members, evict, rebuild the draw buffers, and return the **missing** ids to fetch (already marked in-flight so an overlapping viewport won't re-request them). Returns empty when below the building band or when the chunk set is unchanged.
    pub fn set_viewport(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        deck_zoom: f64,
    ) -> Vec<ChunkId> {
        let update = self
            .chunk_residency
            .set_viewport(min_x, min_y, max_x, max_y, deck_zoom);
        self.apply_draw_rebuild(update.rebuild);
        update.missing
    }
}

impl WorldResidency {
    /// Close the ingest frame at `now_ms`: records stats + evicts + rebuilds via [`Self::end_apply_frame`]. No-op when no frame is open.
    pub fn end_ingest_frame_at(&mut self, now_ms: f64) {
        let rebuild = self.chunk_residency.end_ingest_frame_at(now_ms);
        self.apply_draw_rebuild(rebuild);
    }
}

impl WorldResidency {
    /// `drainFrame` tail — record the frame's apply stats, then evict + rebuild once. `elapsed_ms` is the wall time the caller measured for this frame's ingest loop.
    pub fn end_apply_frame(&mut self, elapsed_ms: f64) {
        let rebuild = self.chunk_residency.end_apply_frame(elapsed_ms);
        self.apply_draw_rebuild(rebuild);
    }
}

impl WorldResidency {
    /// Load + narrow `prefabs.json.gz`: the class table (`has_oversized`) and the u16-keyed building footprint lookup, then the glyph lookup. Returns the prefab count.
    pub fn load_prefabs_gz(&mut self, bytes: &[u8]) -> Result<usize> {
        let rebuild = self.chunk_residency.load_prefabs_gz(bytes)?;
        self.apply_draw_rebuild(rebuild);
        Ok(self.chunk_residency.prefab_count())
    }
}

impl WorldResidency {
    /// `terrain` is the world the caller believes it is loading. The archive records the terrain it was built for and refuses a caller that disagrees — without it, everon's catalogue loaded for arland would resolve every prefab id against the wrong table and still *find* one. Rebuilds the glyph lookup; returns the prefab count.
    pub fn load_prefabs(&mut self, bytes: &[u8], terrain: &str) -> Result<usize> {
        let rebuild = self.chunk_residency.load_prefabs(bytes, terrain)?;
        self.apply_draw_rebuild(rebuild);
        Ok(self.chunk_residency.prefab_count())
    }
}
