//! Role: queries.
//! Position: `streaming/scheduler` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::streaming::loaders::chunk::WorldChunk;
use crate::streaming::scheduler::chunk_math::TerrainSizeM;
use crate::streaming::scheduler::state::WorldResidency;
use crate::world::environment::buildings::prefab::PrefabRow;

impl WorldResidency {
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

impl WorldResidency {
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

impl WorldResidency {
    /// Chunk.
    #[must_use]
    pub fn chunk(&self, id: &str) -> Option<&WorldChunk> {
        self.chunks.get(id)
    }
}

impl WorldResidency {
    /// Terrain.
    #[must_use]
    pub fn terrain(&self) -> TerrainSizeM {
        self.terrain
    }
}

impl WorldResidency {
    /// Chunk size m.
    #[must_use]
    pub fn chunk_size_m(&self) -> f64 {
        self.manifest.as_ref().map_or(
            crate::streaming::loaders::manifest::DEFAULT_CHUNK_SIZE_M,
            |m| m.chunk_size_m,
        )
    }
}

impl WorldResidency {
    /// Prefab rows.
    pub fn prefab_rows(&self) -> impl Iterator<Item = &PrefabRow> {
        self.prefab_by_id.values().map(|e| &e.row)
    }
}
