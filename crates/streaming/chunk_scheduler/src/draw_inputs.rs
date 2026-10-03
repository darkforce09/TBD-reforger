//! The read accessors the draw buffers and the residency statistics compose from.
//!
//! **Role:** exposes, read-only, the residency state the draw buffers read (the viewport and
//! zoom, the content epoch, the pin, the chunk index, the resident chunks, the prefab tables and
//! the importance zooms) and the counters the residency statistics report.
//! **Position:** `chunk_scheduler`; methods of
//! [`crate::state::ChunkResidency`]; read by
//! `chunk_draw_buffers` and the residency tests, so the residency's fields stay private to
//! the scheduler folder.
//! **Signals & state:** none; every accessor borrows or copies.
//! **Invariants:** no accessor mutates; each returns the field it names unchanged.

use prefab_catalog::footprint_lookups::BuildingPrefabInfo;
use prefab_catalog::footprint_lookups::FencePrefabInfo;
use prefab_catalog::prefab_rows::PrefabEntry;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::state::ChunkResidency;
use map_coordinates::chunk_math::Bbox;
use world_chunks::ChunkId;
use world_chunks::world_chunk::WorldChunk;

impl ChunkResidency {
    /// The deck zoom of the last viewport.
    #[must_use]
    pub fn deck_zoom(&self) -> f64 {
        self.deck_zoom
    }

    /// The last viewport, world metres `[min_x, min_y, max_x, max_y]`.
    #[must_use]
    pub fn last_viewport(&self) -> Bbox {
        self.last_viewport
    }

    /// The content epoch: bumped on every chunk insert, eviction and invalidation.
    #[must_use]
    pub fn content_epoch(&self) -> u64 {
        self.content_epoch
    }

    /// The pinned chunk ids, in chunk-math order.
    #[must_use]
    pub fn pinned_ids(&self) -> &[String] {
        &self.pinned_ids
    }

    /// Whether the pin holds chunk `id`.
    #[must_use]
    pub fn is_pinned(&self, id: &ChunkId) -> bool {
        self.pinned_set.contains(id.as_str())
    }

    /// The chunk index's cell ids, once loaded.
    #[must_use]
    pub fn cell_ids(&self) -> Option<&HashSet<String>> {
        self.cell_ids.as_ref()
    }

    /// The resident chunks by id.
    #[must_use]
    pub fn resident_chunks(&self) -> &HashMap<String, WorldChunk> {
        &self.chunks
    }

    /// The prefab table entries (row and class code), in table order.
    pub fn prefab_entries(&self) -> impl Iterator<Item = &PrefabEntry> {
        self.prefab_by_id.values()
    }

    /// The number of distinct prefab ids in the prefab tables.
    #[must_use]
    pub fn prefab_count(&self) -> usize {
        self.prefab_by_id.len()
    }

    /// The building footprint of u16 prefab index `prefab_idx`, when it is a building.
    #[must_use]
    pub fn building_prefab(&self, prefab_idx: u16) -> Option<&BuildingPrefabInfo> {
        self.building_by_u16.get(&prefab_idx)
    }

    /// The fence footprint of u16 prefab index `prefab_idx`, when it is a fence.
    #[must_use]
    pub fn fence_prefab(&self, prefab_idx: u16) -> Option<&FencePrefabInfo> {
        self.fence_by_u16.get(&prefab_idx)
    }

    /// The lowest building importance zoom, when any building has one.
    #[must_use]
    pub fn min_importance_zoom(&self) -> Option<f64> {
        self.min_importance_zoom
    }

    /// The sorted distinct building importance zooms.
    #[must_use]
    pub fn importance_breakpoints(&self) -> &[f64] {
        &self.importance_breakpoints
    }

    /// Chunks inserted since construction.
    #[must_use]
    pub fn chunks_applied(&self) -> u64 {
        self.chunks_applied
    }

    /// Apply frames closed since construction.
    #[must_use]
    pub fn apply_frames(&self) -> u64 {
        self.apply_frames
    }

    /// The longest apply frame, ms.
    #[must_use]
    pub fn max_apply_ms(&self) -> f64 {
        self.max_apply_ms
    }

    /// The last apply frame, ms.
    #[must_use]
    pub fn apply_budget_ms_last(&self) -> f64 {
        self.apply_budget_ms_last
    }

    /// The number of objects in the object index.
    #[must_use]
    pub fn object_index_size(&self) -> usize {
        self.index.size()
    }

    /// The number of chunks parsed with zero instances.
    #[must_use]
    pub fn known_empty_count(&self) -> usize {
        self.known_empty.len()
    }
}
