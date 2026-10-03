//! Picks and lookups over the resident chunks, and the residency's counts and events.
//!
//! **Role:** answers the residency's read questions: nearest and rectangle picks, one chunk, the
//! terrain and chunk size, the prefab rows, the resident and pinned counts, the strict draw set
//! of chunks, and the queued residency events.
//! **Position:** `chunk_scheduler`; methods of
//! [`crate::state::ChunkResidency`]; called through the composed world
//! residency by the loaders, the draw buffers and the residency tests.
//! **Signals & state:** picks rebuild the object index's grid lazily; `take_residency_events`
//! drains the event queue; everything else reads.
//! **Invariants:** the draw set is sorted and lies inside the pinned set and the chunk index.

use crate::state::ChunkResidency;
use crate::state::ResidencyEvent;
use crate::viewport::DRAW_CULL_MARGIN_M;
use map_coordinates::chunk_math::Bbox;
use map_coordinates::chunk_math::TerrainSizeM;
use map_coordinates::chunk_math::chunk_ids_for_rect;
use map_coordinates::chunk_math::chunk_rect_for_bbox;
use prefab_catalog::prefab_rows::PrefabRow;
use world_chunks::ChunkId;
use world_chunks::world_chunk::WorldChunk;

impl ChunkResidency {
    /// Pick nearest world instance id `"{chunkId}:{row}"` within `radius_m`, optional class mask.
    pub fn pick_nearest(
        &mut self,
        x: f64,
        y: f64,
        radius_m: f64,
        mask: Option<u32>,
    ) -> Option<String> {
        self.index.pick_nearest(x, y, radius_m, mask)
    }
}

impl ChunkResidency {
    /// Pick all world instance ids inside a world-meter bbox, optional class mask.
    pub fn pick_rect(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        mask: Option<u32>,
    ) -> Vec<String> {
        self.index.pick_rect(min_x, min_y, max_x, max_y, mask)
    }
}

impl ChunkResidency {
    /// The resident chunk `id`, when it is resident (an empty stub included).
    #[must_use]
    pub fn chunk(&self, id: &ChunkId) -> Option<&WorldChunk> {
        self.chunks.get(id.as_str())
    }
}

impl ChunkResidency {
    /// Terrain.
    #[must_use]
    pub fn terrain(&self) -> TerrainSizeM {
        self.terrain
    }
}

impl ChunkResidency {
    /// Chunk size m.
    #[must_use]
    pub fn chunk_size_m(&self) -> f64 {
        self.manifest
            .as_ref()
            .map_or(world_chunks::terrain_manifest::DEFAULT_CHUNK_SIZE_M, |m| {
                m.chunk_size_m
            })
    }
}

impl ChunkResidency {
    /// Prefab rows.
    pub fn prefab_rows(&self) -> impl Iterator<Item = &PrefabRow> {
        self.prefab_by_id.values().map(|e| &e.row)
    }
}

impl ChunkResidency {
    /// Take residency events.
    pub fn take_residency_events(&mut self) -> Vec<ResidencyEvent> {
        std::mem::take(&mut self.residency_events)
    }
}

impl ChunkResidency {
    /// Resident chunk ids (sorted) — parity/debug surface.
    #[must_use]
    pub fn resident_chunk_ids(&self) -> Vec<String> {
        let mut v: Vec<String> = self.chunks.keys().cloned().collect();
        v.sort();
        v
    }
}

impl ChunkResidency {
    /// Total building instances across the pinned chunks (== JS `getWorldBuildings().length`).
    #[must_use]
    pub fn pinned_building_count(&self) -> u32 {
        self.pinned_ids
            .iter()
            .map(|id| self.building_counts.get(id).copied().unwrap_or(0))
            .sum()
    }
}

impl ChunkResidency {
    /// Chunks resident.
    #[must_use]
    pub fn chunks_resident(&self) -> usize {
        self.chunks.len()
    }
}

impl ChunkResidency {
    /// Instance count of a resident chunk (`None` if not resident).
    #[must_use]
    pub fn resident_instance_count(&self, id: &ChunkId) -> Option<u32> {
        self.chunks.get(id.as_str()).map(|c| c.count)
    }
}

impl ChunkResidency {
    /// The strict draw set under `strict_bbox`: its chunks (no cull margin) that the chunk index lists and the pin holds, sorted. Empty before the manifest loads.
    #[must_use]
    pub fn draw_chunk_ids(&self, strict_bbox: Bbox) -> Vec<String> {
        debug_assert_eq!(DRAW_CULL_MARGIN_M, 0.0);
        let chunk_size_m = match &self.manifest {
            Some(m) => m.chunk_size_m,
            None => return Vec::new(),
        };
        let rect = chunk_rect_for_bbox(strict_bbox, self.terrain, chunk_size_m);
        let mut ids = chunk_ids_for_rect(rect);
        if let Some(cells) = &self.cell_ids {
            ids.retain(|id| cells.contains(id));
        }
        ids.retain(|id| self.pinned_set.contains(id));
        ids.sort();
        ids
    }
}
