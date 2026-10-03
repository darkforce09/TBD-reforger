//! Chunk ingest into the residency: the loads and the chunk ingest of [`WorldResidency`].
//!
//! **Role:** feeds the chunk scheduler's residency: the terrain manifest (and the terrain size
//! from `worldBounds`), the prefab tables from either catalogue form, the chunk index whose cells
//! bound every pin, and one chunk at a time from gzip JSON or `TBDC` binary.
//! **Position:** `streaming/scheduler`; reads the parsers of `crate::streaming::loaders`; called
//! by the world loader, the debug world line-of-sight bench and the residency tests.
//! **Signals & state:** mutates the [`WorldResidency`] it is called on (prefab tables, chunk
//! index, resident chunks, known-empty and fetch-failure sets).
//! **Invariants:** an ingest answers `Applied`, `ParsedEmpty` (known-empty from then on) or a
//! shape mismatch that counts toward the fetch-failure cap; a chunk that fails to parse inserts
//! nothing.

use crate::streaming::scheduler::state::IngestOutcome;
use crate::streaming::scheduler::state::WorldResidency;
use map_coordinates::chunk_math::TerrainSizeM;
use prefab_catalog::prefab_tables::PrefabTables;
use prefab_catalog::prefab_tables::tables_from_bytes;
use prefab_catalog::prefab_tables::tables_from_json;
use prefab_catalog::world_payload::WorldError;
use prefab_catalog::world_payload::bytes_to_json;
use serde_json::Value;
use std::collections::HashSet;
use world_chunks::ChunkId;
use world_chunks::chunk_container::ChunkBinError;
use world_chunks::chunk_container::parse_chunk_bin_for;
use world_chunks::terrain_manifest::narrow_cells;
use world_chunks::terrain_manifest::parse_objects_manifest;
use world_chunks::world_chunk::WorldChunk;
use world_chunks::world_chunk::parse_chunk;

impl WorldResidency {
    /// Parse the terrain manifest: the `objects` block (chunk size + object-export gate) and the top-level `worldBounds` (terrain extent for the chunk-id math).
    pub fn load_manifest_json(&mut self, json: &str) -> Result<(), WorldError> {
        let raw: Value = serde_json::from_str(json).map_err(|e| WorldError::Json(e.to_string()))?;
        self.manifest = Some(parse_objects_manifest(&raw).ok_or(WorldError::Manifest)?);
        if let Some(b) = raw.get("worldBounds").and_then(Value::as_array) {
            let v: Vec<f64> = b.iter().filter_map(Value::as_f64).collect();
            if v.len() == 4 {
                self.terrain = TerrainSizeM {
                    width: v[2] - v[0],
                    height: v[3] - v[1],
                };
            }
        }
        Ok(())
    }
}

impl WorldResidency {
    /// Load + narrow `prefabs.json.gz`: the class table (`has_oversized`) and the u16-keyed building footprint lookup. Returns the prefab count.
    pub fn load_prefabs_gz(&mut self, bytes: &[u8]) -> Result<usize, WorldError> {
        let tables = tables_from_json(&bytes_to_json(bytes)?)?;
        self.apply_prefab_tables(tables);
        Ok(self.prefab_by_id.len())
    }
}

impl WorldResidency {
    /// `terrain` is the world the caller believes it is loading. The archive records the terrain it was built for and refuses a caller that disagrees — without it, everon's catalogue loaded for arland would resolve every prefab id against the wrong table and still *find* one.
    pub fn load_prefabs(&mut self, bytes: &[u8], terrain: &str) -> Result<usize, WorldError> {
        let tables = tables_from_bytes(bytes, terrain)?;
        self.apply_prefab_tables(tables);
        Ok(self.prefab_by_id.len())
    }
}

impl WorldResidency {
    /// Apply prefab tables.
    pub(crate) fn apply_prefab_tables(&mut self, tables: PrefabTables) {
        self.prefab_by_id = tables.by_id;
        self.has_oversized = tables.has_oversized;
        self.building_by_u16 = tables.building_by_u16;
        self.fence_by_u16 = tables.fence_by_u16;
        self.min_importance_zoom = tables.min_importance_zoom;
        self.importance_breakpoints = tables.importance_breakpoints;
        self.rebuild_glyph_lookup_from_prefabs();
    }
}

impl WorldResidency {
    /// Load the chunk-index (`objects/chunks/manifest.json`) `cells[]` → the existing-chunk id set the viewport request is intersected with. Returns the cell count.
    pub fn load_chunk_index_json(&mut self, json: &str) -> Result<usize, WorldError> {
        let raw: Value = serde_json::from_str(json).map_err(|e| WorldError::Json(e.to_string()))?;
        let cells = narrow_cells(&raw).unwrap_or_default();
        let set: HashSet<String> = cells.into_iter().map(|c| c.id.into_inner()).collect();
        let n = set.len();
        self.cell_ids = Some(set);
        Ok(n)
    }
}

impl WorldResidency {
    /// Ingest chunk gz.
    pub fn ingest_chunk_gz(&mut self, id: &str, bytes: &[u8]) -> Result<IngestOutcome, WorldError> {
        let raw = bytes_to_json(bytes)?;
        match parse_chunk(&ChunkId::from(id), &raw, &self.prefab_by_id) {
            Some(chunk) => Ok(self.apply_parsed_chunk(id, chunk)),
            None => {
                self.note_fetch_failure(id);
                Ok(IngestOutcome::ShapeMismatch)
            }
        }
    }
}

impl WorldResidency {
    /// Ingest chunk bin.
    pub fn ingest_chunk_bin(
        &mut self,
        id: &str,
        bytes: &[u8],
    ) -> Result<IngestOutcome, ChunkBinError> {
        let chunk = parse_chunk_bin_for(&ChunkId::from(id), bytes)?;
        Ok(self.apply_parsed_chunk(id, chunk))
    }
}

impl WorldResidency {
    /// Apply parsed chunk.
    pub(crate) fn apply_parsed_chunk(&mut self, id: &str, chunk: WorldChunk) -> IngestOutcome {
        let count = chunk.count;
        self.insert_chunk(id, chunk);
        self.fetch_failures.remove(id);
        if count > 0 {
            self.known_empty.remove(id);
            IngestOutcome::Applied(count)
        } else {
            self.known_empty.insert(id.to_string());
            IngestOutcome::ParsedEmpty
        }
    }
}

#[cfg(test)]
#[path = "tests/chunk_ingest_prefab_lane_tests.rs"]
mod prefab_lane_tests;

#[cfg(test)]
#[path = "tests/chunk_ingest_chunk_bin_tests.rs"]
mod chunk_bin_tests;
