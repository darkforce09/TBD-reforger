//! The headless world store: one terrain's served world data read into memory, one chunk at a
//! time.
//!
//! **Role:** [`WorldStore`] holds the terrain manifest's `objects` block, the prefab lookup, the
//! road network, the land-cover regions and the last object chunk parsed, and reports the counts
//! each load keeps.
//! **Position:** over `world_chunks` (chunk rows, the terrain manifest, chunk ids),
//! `prefab_catalog` (prefab rows, payload decoding), `road_network` (segments, the airfield box)
//! and `vegetation` (regions). The map engine's browser world loader and the developer tools'
//! export, raster and verification pipelines read it.
//! **Signals & state:** the store's own fields, replaced by each load; no shared state.
//! **Invariants:** a road payload is told apart by its first bytes (empty is refused, `1f 8b` is
//! gzip JSON, anything else is the validating archive reader) and a refused load leaves the roads
//! as they were; the binary and JSON road lanes build the same network.

use std::collections::HashMap;

use map_coordinates::chunk_math::Bbox;
use prefab_catalog::prefab_rows::PrefabEntry;
use prefab_catalog::prefab_rows::build_prefab_maps;
use prefab_catalog::prefab_rows::narrow_prefab_rows;
use prefab_catalog::world_payload::WorldError;
use prefab_catalog::world_payload::bytes_to_json;
use road_network::airfield::compute_airfield_bbox;
use road_network::network::RoadSegment;
use road_network::network::parse_roads_payload;
use road_network::network::road_network_from_bytes;
use serde_json::Value;
use vegetation::regions::LandCoverRegion;
use vegetation::regions::parse_regions_payload;
use world_chunks::ChunkId;
use world_chunks::terrain_manifest::ObjectsManifest;
use world_chunks::terrain_manifest::parse_objects_manifest;
use world_chunks::world_chunk::WorldChunk;
use world_chunks::world_chunk::parse_chunk;

use crate::error::Result;

/// One terrain's world data in memory: the manifest's `objects` block, the prefab lookup, the
/// roads, the land-cover regions and the last parsed chunk.
#[derive(Default)]
pub struct WorldStore {
    /// The terrain manifest's `objects` block, once loaded.
    pub manifest: Option<ObjectsManifest>,

    /// The prefab rows, keyed by the bits of their `f64` prefab id.
    pub prefab_by_id: HashMap<u64, PrefabEntry>,

    /// Whether any classified prefab has a half-extent of 64 m or more.
    pub has_oversized: bool,

    /// The road network's segments.
    pub roads: Vec<RoadSegment>,

    /// The land-cover regions.
    pub regions: Vec<LandCoverRegion>,

    /// The last chunk [`WorldStore::parse_chunk_gz`] parsed.
    pub last_chunk: Option<WorldChunk>,

    /// How many chunks have parsed with an `instances` array.
    pub chunks_loaded: usize,
}

impl WorldStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse the terrain manifest's `objects` block.
    ///
    /// # Errors
    /// [`WorldError::Json`] when the manifest is not JSON and [`WorldError::Manifest`] when it
    /// lacks the object-export paths, both as [`Error::Payload`](crate::Error::Payload).
    pub fn load_manifest_json(&mut self, json: &str) -> Result<()> {
        let raw: Value = serde_json::from_str(json).map_err(|e| WorldError::Json(e.to_string()))?;
        self.manifest = Some(parse_objects_manifest(&raw).ok_or(WorldError::Manifest)?);
        Ok(())
    }

    /// Load + narrow `prefabs.json.gz`, building the prefab lookup + `has_oversized`. Returns the prefab count.
    ///
    /// # Errors
    /// [`Error::Payload`](crate::Error::Payload) when the payload does not inflate or parse, or a
    /// row's `prefabId` is not a whole number in `0..=u32::MAX`.
    pub fn load_prefabs_gz(&mut self, bytes: &[u8]) -> Result<usize> {
        let raw = bytes_to_json(bytes)?;
        let rows = narrow_prefab_rows(&raw).map_err(WorldError::from)?;
        let (by_id, has_oversized) = build_prefab_maps(rows);
        self.prefab_by_id = by_id;
        self.has_oversized = has_oversized;
        Ok(self.prefab_by_id.len())
    }

    /// Parse one `objects/chunks/{id}.json.gz` into `last_chunk`. Returns its instance count (0 when the payload has no `instances` array).
    ///
    /// # Errors
    /// [`Error::Payload`](crate::Error::Payload) when the payload does not inflate or parse.
    pub fn parse_chunk_gz(&mut self, id: &ChunkId, bytes: &[u8]) -> Result<u32> {
        let raw = bytes_to_json(bytes)?;
        let chunk = parse_chunk(id, &raw, &self.prefab_by_id);
        let count = chunk.as_ref().map_or(0, |c| c.count);
        if chunk.is_some() {
            self.chunks_loaded += 1;
        }
        self.last_chunk = chunk;
        Ok(count)
    }

    /// Load + centerline `roads.json.gz`. Returns the kept segment count.
    ///
    /// # Errors
    /// [`Error::Payload`](crate::Error::Payload) when the payload does not inflate or parse.
    pub fn load_roads_gz(&mut self, bytes: &[u8]) -> Result<usize> {
        let raw = bytes_to_json(bytes)?;
        self.roads = parse_roads_payload(&raw);
        Ok(self.roads.len())
    }

    /// Load the road network from either of its served forms, told apart by the first bytes:
    ///
    /// | first bytes | route |
    /// |---|---|
    /// | *none* | [`WorldError::EmptyPayload`]: an empty buffer has no format to sniff |
    /// | `1f 8b` | gzip: [`load_roads_gz`](Self::load_roads_gz), the JSON path |
    /// | anything else | [`road_network_from_bytes`], the validating rkyv reader |
    ///
    /// Returns the kept segment count.
    ///
    /// # Errors
    /// [`Error::Payload`](crate::Error::Payload) for an empty buffer or a gzip payload that does
    /// not inflate or parse, and [`Error::RoadArchive`](crate::Error::RoadArchive) for any other
    /// buffer the archive reader refuses.
    pub fn load_roads(&mut self, bytes: &[u8]) -> Result<usize> {
        if bytes.is_empty() {
            return Err(WorldError::EmptyPayload.into());
        }
        if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            return self.load_roads_gz(bytes);
        }
        self.roads = road_network_from_bytes(bytes)?;
        Ok(self.roads.len())
    }

    /// Load + narrow `forest-regions.json.gz`. Returns the kept region count.
    ///
    /// # Errors
    /// [`Error::Payload`](crate::Error::Payload) when the payload does not inflate or parse.
    pub fn load_forest_regions_gz(&mut self, bytes: &[u8]) -> Result<usize> {
        let raw = bytes_to_json(bytes)?;
        self.regions = parse_regions_payload(&raw);
        Ok(self.regions.len())
    }

    /// Declared total instance count from the manifest (`objects.instanceCount`), if loaded.
    #[must_use]
    pub fn instance_count_total(&self) -> f64 {
        self.manifest
            .as_ref()
            .and_then(|m| m.instance_count)
            .unwrap_or(0.0)
    }

    /// The loaded road segments whose `road_class` is `runway`, in load order; empty before the
    /// roads load.
    #[must_use]
    pub fn runway_segments(&self) -> Vec<&RoadSegment> {
        self.roads
            .iter()
            .filter(|r| r.road_class == "runway")
            .collect()
    }

    /// NW airfield bbox from runway union + 30 m margin.
    #[must_use]
    pub fn airfield_bbox(&self) -> Option<Bbox> {
        let runways: Vec<&RoadSegment> = self.runway_segments();
        compute_airfield_bbox(&runways.into_iter().cloned().collect::<Vec<RoadSegment>>())
    }
}

#[cfg(test)]
#[path = "tests/store_tests.rs"]
mod tests;
