//! The chunk residency: which world chunks are pinned, in flight, resident and evictable, and the
//! prefab tables and object index over them.
//!
//! **Role:** holds the state of the chunk scheduler: the terrain manifest and size, the prefab
//! tables, the resident chunks with their LRU clocks, the viewport pin, the in-flight marks, the
//! known-empty and fetch-failure sets, the object index, the residency events and the ingest-frame
//! statistics.
//! **Position:** `chunk_scheduler`; driven by the composed world residency of
//! `chunk_draw_buffers::world_residency`, which also owns the draw buffers composed from
//! this state; the other scheduler files add its methods.
//! **Signals & state:** every field is private to the scheduler folder; other folders read it
//! through the accessors of `draw_inputs.rs` and `queries.rs` and change it only through the
//! scheduler's methods.
//! **Invariants:** a chunk id is resident, in flight or neither, never both resident and in
//! flight after an insert; pinned and known-empty chunks are never evicted; `content_epoch`
//! grows on every insert, eviction and invalidation.

use prefab_catalog::footprint_lookups::BuildingPrefabInfo;
use prefab_catalog::footprint_lookups::FencePrefabInfo;
use prefab_catalog::prefab_rows::PrefabEntry;

use crate::world_object_index::WorldSpatialIndex;
use map_coordinates::chunk_math::Bbox;
use map_coordinates::chunk_math::TerrainSizeM;
use std::collections::HashMap;
use std::collections::HashSet;
use world_chunks::ChunkId;
use world_chunks::terrain_manifest::ObjectsManifest;
use world_chunks::world_chunk::WorldChunk;
use world_file_formats::ids::PrefabId;

/// Ingest outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestOutcome {
    /// Parsed + inserted with this many instances.
    Applied(u32),

    /// Well-formed chunk JSON with zero instances — stub kept, marked known-empty forever.
    ParsedEmpty,

    /// Payload did not match the chunk shape — counted toward the retry cap.
    ShapeMismatch,
}

/// Residency event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResidencyEvent {
    /// A chunk was parsed and inserted (an empty stub counts; the mirror ignores it).
    Inserted(ChunkId),

    /// A chunk was evicted or invalidated.
    Evicted(ChunkId),
}

/// Multi-chunk residency: pin, in-flight marks, fetch-failure cap, LRU eviction, ingest budget
/// and the world object index over the resident chunks.
pub struct ChunkResidency {
    /// The terrain manifest's `objects` block, once loaded.
    pub(super) manifest: Option<ObjectsManifest>,

    /// The terrain extent the chunk-id math clamps to.
    pub(super) terrain: TerrainSizeM,

    /// Prefab rows by prefab id.
    pub(super) prefab_by_id: HashMap<PrefabId, PrefabEntry>,

    /// Whether a prefab is larger than a chunk (the pin then grows by one ring).
    pub(super) has_oversized: bool,

    /// Building footprints by u16 prefab index.
    pub(super) building_by_u16: HashMap<u16, BuildingPrefabInfo>,

    /// The lowest building importance zoom, when any building has one.
    pub(super) min_importance_zoom: Option<f64>,

    /// The sorted distinct building importance zooms.
    pub(super) importance_breakpoints: Vec<f64>,

    /// Fence footprints by u16 prefab index.
    pub(super) fence_by_u16: HashMap<u16, FencePrefabInfo>,

    /// The chunk index's cell ids, once loaded; every pin is intersected with it.
    pub(super) cell_ids: Option<HashSet<String>>,

    /// Resident chunks by id.
    pub(super) chunks: HashMap<String, WorldChunk>,

    /// Building rows with a known footprint, per resident chunk.
    pub(super) building_counts: HashMap<String, u32>,

    /// LRU clock of each resident chunk's last use.
    pub(super) last_used: HashMap<String, u64>,

    /// Insert order of each resident chunk (the LRU tie-break).
    pub(super) inserted_seq: HashMap<String, u64>,

    /// The LRU clock.
    pub(super) use_tick: u64,

    /// The insert counter.
    pub(super) insert_counter: u64,

    /// The pinned chunk ids, in chunk-math order.
    pub(super) pinned_ids: Vec<String>,

    /// The pinned chunk ids as a set.
    pub(super) pinned_set: HashSet<String>,

    /// The joined pinned ids: tells an unchanged pin from a new one.
    pub(super) pinned_key: String,

    /// Chunk ids requested and not yet resident.
    pub(super) inflight: HashSet<String>,

    /// The object index over the resident chunks.
    pub(super) index: WorldSpatialIndex,

    /// Every eviction victim since construction, in order.
    pub(super) eviction_log: Vec<String>,

    /// Inserted and evicted chunk events, queued until taken.
    pub(super) residency_events: Vec<ResidencyEvent>,

    /// Chunks inserted since construction.
    pub(super) chunks_applied: u64,

    /// Apply frames closed since construction.
    pub(super) apply_frames: u64,

    /// The longest apply frame, ms.
    pub(super) max_apply_ms: f64,

    /// Apply frames that ran past the budget.
    pub(super) frames_over_budget: u64,

    /// The last apply frame, ms.
    pub(super) apply_budget_ms_last: f64,

    /// Bumped on every insert, eviction and invalidation.
    pub(super) content_epoch: u64,

    /// Chunks parsed with zero instances: never requested again, never evicted.
    pub(super) known_empty: HashSet<String>,

    /// Failed fetches per chunk id since the current pin.
    pub(super) fetch_failures: HashMap<String, u8>,

    /// Start of the open ingest frame, ms, when one is open.
    pub(super) ingest_frame_start_ms: Option<f64>,

    /// The deck zoom of the last viewport.
    pub(super) deck_zoom: f64,

    /// The last viewport, world metres `[min_x, min_y, max_x, max_y]`.
    pub(super) last_viewport: Bbox,
}

impl Default for ChunkResidency {
    fn default() -> Self {
        Self {
            manifest: None,
            terrain: TerrainSizeM::default(),
            prefab_by_id: HashMap::new(),
            has_oversized: false,
            building_by_u16: HashMap::new(),
            min_importance_zoom: None,
            importance_breakpoints: Vec::new(),
            fence_by_u16: HashMap::new(),
            cell_ids: None,
            chunks: HashMap::new(),
            building_counts: HashMap::new(),
            last_used: HashMap::new(),
            inserted_seq: HashMap::new(),
            use_tick: 0,
            insert_counter: 0,
            pinned_ids: Vec::new(),
            pinned_set: HashSet::new(),
            pinned_key: String::new(),
            inflight: HashSet::new(),
            index: WorldSpatialIndex::default(),
            eviction_log: Vec::new(),
            residency_events: Vec::new(),
            chunks_applied: 0,
            apply_frames: 0,
            max_apply_ms: 0.0,
            frames_over_budget: 0,
            apply_budget_ms_last: 0.0,
            content_epoch: 0,
            known_empty: HashSet::new(),
            fetch_failures: HashMap::new(),
            ingest_frame_start_ms: None,
            deck_zoom: -2.0,
            last_viewport: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

impl ChunkResidency {
    /// An empty residency: no manifest, no prefabs, nothing pinned or resident.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
