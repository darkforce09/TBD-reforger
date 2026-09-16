//! Role: store.
//! Position: `streaming/loaders` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::collections::HashMap;
use std::io::Read;

use serde_json::Value;

use crate::streaming::loaders::chunk::WorldChunk;
use crate::streaming::loaders::chunk::parse_chunk;
use crate::streaming::loaders::manifest::ObjectsManifest;
use crate::streaming::loaders::manifest::parse_objects_manifest;
use crate::streaming::scheduler::chunk_math::Bbox;
use crate::world::environment::buildings::prefab::PrefabEntry;
use crate::world::environment::buildings::prefab::build_prefab_maps;
use crate::world::environment::buildings::prefab::narrow_prefab_rows;
use crate::world::environment::vegetation::regions::LandCoverRegion;
use crate::world::environment::vegetation::regions::parse_regions_payload;
use crate::world::terrain::roads::airfield::compute_airfield_bbox;
use crate::world::terrain::roads::network::RoadSegment;
use crate::world::terrain::roads::network::parse_roads_payload;
use crate::world::terrain::roads::network::road_network_from_bytes;

/// A world-parse failure (gunzip, JSON, a binary archive, or a manifest missing the object-export paths).
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    /// Gzip.
    #[error("world: gzip inflate failed: {0}")]
    Gzip(String),

    /// Json.
    #[error("world: json parse failed: {0}")]
    Json(String),

    /// Manifest.
    #[error("world: manifest missing objects/prefabsPath/chunksPath")]
    Manifest,

    /// [`BinaryError`]: super::binary::BinaryError.
    #[error("world: binary archive failed to load: {0}")]
    Archive(String),

    /// A zero-length payload. Named rather than folded into [`WorldError::Json`] because the format sniff cannot even *classify* an empty buffer, and "json parse failed: EOF while parsing a value" is a misleading thing to say about a file that was never fetched.
    #[error("world: empty payload — no format to sniff")]
    EmptyPayload,
}

/// Bytes to json.
pub fn bytes_to_json(bytes: &[u8]) -> Result<Value, WorldError> {
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut decoder = flate2::read::GzDecoder::new(bytes);
        let mut inflated = Vec::new();
        decoder
            .read_to_end(&mut inflated)
            .map_err(|e| WorldError::Gzip(e.to_string()))?;
        serde_json::from_slice(&inflated).map_err(|e| WorldError::Json(e.to_string()))
    } else {
        serde_json::from_slice(bytes).map_err(|e| WorldError::Json(e.to_string()))
    }
}

/// Manifest + prefab table + roads + regions + the last parsed chunk.
#[derive(Default)]
pub struct WorldStore {
    /// Manifest.
    pub manifest: Option<ObjectsManifest>,

    /// Prefab by id.
    pub prefab_by_id: HashMap<u64, PrefabEntry>,

    /// Has oversized.
    pub has_oversized: bool,

    /// Roads.
    pub roads: Vec<RoadSegment>,

    /// Regions.
    pub regions: Vec<LandCoverRegion>,

    /// Last chunk.
    pub last_chunk: Option<WorldChunk>,

    /// Chunks loaded.
    pub chunks_loaded: usize,
}

impl WorldStore {
    /// New.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse the terrain manifest's `objects` block.
    pub fn load_manifest_json(&mut self, json: &str) -> Result<(), WorldError> {
        let raw: Value = serde_json::from_str(json).map_err(|e| WorldError::Json(e.to_string()))?;
        self.manifest = Some(parse_objects_manifest(&raw).ok_or(WorldError::Manifest)?);
        Ok(())
    }

    /// Load + narrow `prefabs.json.gz`, building the prefab lookup + `has_oversized`. Returns the prefab count.
    pub fn load_prefabs_gz(&mut self, bytes: &[u8]) -> Result<usize, WorldError> {
        let raw = bytes_to_json(bytes)?;
        let (by_id, has_oversized) = build_prefab_maps(narrow_prefab_rows(&raw));
        self.prefab_by_id = by_id;
        self.has_oversized = has_oversized;
        Ok(self.prefab_by_id.len())
    }

    /// Parse one `objects/chunks/{id}.json.gz` into `last_chunk`. Returns its instance count (0 when the payload has no `instances` array).
    pub fn parse_chunk_gz(&mut self, id: &str, bytes: &[u8]) -> Result<u32, WorldError> {
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
    pub fn load_roads_gz(&mut self, bytes: &[u8]) -> Result<usize, WorldError> {
        let raw = bytes_to_json(bytes)?;
        self.roads = parse_roads_payload(&raw);
        Ok(self.roads.len())
    }

    /// | first bytes | route | |---|---| | *none* | [`WorldError::EmptyPayload`] — an empty buffer has no format to sniff | | `1f 8b` | gzip: [`load_roads_gz`](Self::load_roads_gz), the JSON path, unchanged | | anything else | [`road_network_from_bytes`], the validating rkyv reader |.
    pub fn load_roads(&mut self, bytes: &[u8]) -> Result<usize, WorldError> {
        if bytes.is_empty() {
            return Err(WorldError::EmptyPayload);
        }
        if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            return self.load_roads_gz(bytes);
        }
        self.roads =
            road_network_from_bytes(bytes).map_err(|e| WorldError::Archive(e.to_string()))?;
        Ok(self.roads.len())
    }

    /// Load + narrow `forest-regions.json.gz`. Returns the kept region count.
    pub fn load_forest_regions_gz(&mut self, bytes: &[u8]) -> Result<usize, WorldError> {
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

    /// Runway segments.
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
