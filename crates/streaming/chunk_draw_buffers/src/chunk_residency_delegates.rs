//! The world residency's chunk residency calls that leave the draw buffers as they are.
//!
//! **Role:** keeps every public chunk residency method callable on the world residency under its
//! own name: in-flight and failure bookkeeping, the ingest frame, the loads and ingests that
//! request no rebuild, the picks, lookups, counts and events.
//! **Position:** `chunk_draw_buffers`; methods of
//! [`crate::world_residency::WorldResidency`], each forwarding to
//! [`chunk_scheduler::state::ChunkResidency`]; called by the loaders, the debug world
//! line-of-sight bench and the residency tests.
//! **Signals & state:** the chunk residency's state, through its own methods.
//! **Invariants:** none of these calls changes anything the draw buffers read until the next
//! rebuild request: an insert or invalidation is picked up by the frame close or viewport pass
//! that follows, exactly as the residency schedules it.

use crate::error::Result;
use crate::world_residency::WorldResidency;
use chunk_scheduler::state::IngestOutcome;
use chunk_scheduler::state::ResidencyEvent;
use map_coordinates::chunk_math::Bbox;
use map_coordinates::chunk_math::TerrainSizeM;
use prefab_catalog::prefab_rows::PrefabRow;
use world_chunks::ChunkId;
use world_chunks::world_chunk::WorldChunk;

impl WorldResidency {
    /// Counts one failed fetch of chunk `id`; at the failure cap it becomes an empty stub.
    pub fn note_fetch_failure(&mut self, id: &ChunkId) {
        self.chunk_residency.note_fetch_failure(id);
    }

    /// Cache a requested-but-undelivered chunk (missing/empty file) as hydrated-empty so it is never re-requested (`requestMissing`'s `applied.set(id, [])`).
    pub fn note_undelivered(&mut self, id: &ChunkId) {
        self.chunk_residency.note_undelivered(id);
    }

    /// Ordered eviction victims since construction — parity surface (Class S eviction-order log).
    #[must_use]
    pub fn eviction_log(&self) -> Vec<String> {
        self.chunk_residency.eviction_log()
    }

    /// Inflight count.
    #[must_use]
    pub fn inflight_count(&self) -> usize {
        self.chunk_residency.inflight_count()
    }

    /// Clear inflight.
    pub fn clear_inflight(&mut self) {
        self.chunk_residency.clear_inflight();
    }

    /// Release one in-flight mark after a soft fetch failure (host may retry next settle).
    pub fn release_inflight(&mut self, id: &ChunkId) {
        self.chunk_residency.release_inflight(id);
    }

    /// Drop a resident (or empty-stub) chunk so the next `set_viewport` re-requests it. Used by the Leptos host to recover from a soft HTTP failure that must not be cached as a permanent empty stub (tree-glyph zoom probes need real instance rows).
    pub fn invalidate_chunk(&mut self, id: &ChunkId) {
        self.chunk_residency.invalidate_chunk(id);
    }

    /// Mark ids as in-flight (not yet resident). Used after `clear_inflight` when starting a replacement fetch so concurrent same-key `set_viewport` does not re-queue them.
    pub fn mark_inflight(&mut self, ids: &[ChunkId]) {
        self.chunk_residency.mark_inflight(ids);
    }

    /// True when every pinned id is either resident or known-empty (present in `chunks`). Empty pin set (gate closed) counts as settled.
    #[must_use]
    pub fn pin_settled(&self) -> bool {
        self.chunk_residency.pin_settled()
    }
}

impl WorldResidency {
    /// Begin ingest frame at.
    pub fn begin_ingest_frame_at(&mut self, now_ms: f64) {
        self.chunk_residency.begin_ingest_frame_at(now_ms);
    }

    /// True once the current ingest frame has consumed the apply budget. `false` when no frame is open (callers may ingest at least one chunk per frame regardless — the JS loop shape).
    #[must_use]
    pub fn ingest_budget_exhausted_at(&self, now_ms: f64) -> bool {
        self.chunk_residency.ingest_budget_exhausted_at(now_ms)
    }

    /// Frames over budget.
    #[must_use]
    pub fn frames_over_budget(&self) -> u64 {
        self.chunk_residency.frames_over_budget()
    }
}

impl WorldResidency {
    /// Parse the terrain manifest: the `objects` block (chunk size + object-export gate) and the top-level `worldBounds` (terrain extent for the chunk-id math).
    pub fn load_manifest_json(&mut self, json: &str) -> Result<()> {
        Ok(self.chunk_residency.load_manifest_json(json)?)
    }

    /// Load the chunk-index (`objects/chunks/manifest.json`) `cells[]` → the existing-chunk id set the viewport request is intersected with. Returns the cell count.
    pub fn load_chunk_index_json(&mut self, json: &str) -> Result<usize> {
        Ok(self.chunk_residency.load_chunk_index_json(json)?)
    }

    /// Ingests chunk `id` from its gzip (or plain) JSON body; see
    /// [`chunk_scheduler::state::ChunkResidency::ingest_chunk_gz`].
    pub fn ingest_chunk_gz(&mut self, id: &ChunkId, bytes: &[u8]) -> Result<IngestOutcome> {
        Ok(self.chunk_residency.ingest_chunk_gz(id, bytes)?)
    }

    /// Ingests chunk `id` from its `TBDC` container; see
    /// [`chunk_scheduler::state::ChunkResidency::ingest_chunk_bin`].
    pub fn ingest_chunk_bin(&mut self, id: &ChunkId, bytes: &[u8]) -> Result<IngestOutcome> {
        Ok(self.chunk_residency.ingest_chunk_bin(id, bytes)?)
    }
}

impl WorldResidency {
    /// Pick nearest world instance id `"{chunkId}:{row}"` within `radius_m`, optional class mask.
    pub fn pick_nearest(
        &mut self,
        x: f64,
        y: f64,
        radius_m: f64,
        mask: Option<u32>,
    ) -> Option<String> {
        self.chunk_residency.pick_nearest(x, y, radius_m, mask)
    }

    /// Pick all world instance ids inside a world-meter bbox, optional class mask.
    pub fn pick_rect(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        mask: Option<u32>,
    ) -> Vec<String> {
        self.chunk_residency
            .pick_rect(min_x, min_y, max_x, max_y, mask)
    }

    /// The resident chunk `id`, when it is resident (an empty stub included).
    #[must_use]
    pub fn chunk(&self, id: &ChunkId) -> Option<&WorldChunk> {
        self.chunk_residency.chunk(id)
    }

    /// Terrain.
    #[must_use]
    pub fn terrain(&self) -> TerrainSizeM {
        self.chunk_residency.terrain()
    }

    /// Chunk size m.
    #[must_use]
    pub fn chunk_size_m(&self) -> f64 {
        self.chunk_residency.chunk_size_m()
    }

    /// Prefab rows.
    pub fn prefab_rows(&self) -> impl Iterator<Item = &PrefabRow> {
        self.chunk_residency.prefab_rows()
    }

    /// Take residency events.
    pub fn take_residency_events(&mut self) -> Vec<ResidencyEvent> {
        self.chunk_residency.take_residency_events()
    }

    /// Resident chunk ids (sorted) — parity/debug surface.
    #[must_use]
    pub fn resident_chunk_ids(&self) -> Vec<String> {
        self.chunk_residency.resident_chunk_ids()
    }

    /// Total building instances across the pinned chunks (== JS `getWorldBuildings().length`).
    #[must_use]
    pub fn pinned_building_count(&self) -> u32 {
        self.chunk_residency.pinned_building_count()
    }

    /// Chunks resident.
    #[must_use]
    pub fn chunks_resident(&self) -> usize {
        self.chunk_residency.chunks_resident()
    }

    /// Instance count of a resident chunk (`None` if not resident).
    #[must_use]
    pub fn resident_instance_count(&self, id: &ChunkId) -> Option<u32> {
        self.chunk_residency.resident_instance_count(id)
    }

    /// Draw chunk ids: the strict draw set under `strict_bbox` (no cull margin) inside the chunk index and the pin, sorted.
    #[must_use]
    pub fn draw_chunk_ids(&self, strict_bbox: Bbox) -> Vec<String> {
        self.chunk_residency.draw_chunk_ids(strict_bbox)
    }
}
