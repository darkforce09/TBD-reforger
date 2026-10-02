//! Role: state.
//! Position: `streaming/scheduler` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::environment::buildings::obb::BuildingPrefabInfo;
use crate::world::environment::buildings::obb::FencePrefabInfo;
use crate::world::environment::buildings::prefab::PrefabEntry;

use crate::spatial::indexing::world::WorldSpatialIndex;
use crate::streaming::loaders::chunk::WorldChunk;
use crate::streaming::loaders::manifest::ObjectsManifest;
use crate::streaming::scheduler::chunk_math::Bbox;
use crate::streaming::scheduler::chunk_math::TerrainSizeM;
use std::collections::HashMap;
use std::collections::HashSet;

/// Glyph prefab info.
#[derive(Clone, Debug)]
pub(crate) struct GlyphPrefabInfo {
    /// Glyph idx.
    pub(crate) glyph_idx: u16,

    /// Size m.
    pub(crate) size_m: f32,

    /// Tint.
    pub(crate) tint: u32,

    /// Group.
    pub(crate) group: u8,
}

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
    Inserted(String),

    /// A chunk was evicted or invalidated.
    Evicted(String),
}

/// Multi-chunk residency + LRU + world spatial index + building/glyph GPU-buffer composer.
pub struct WorldResidency {
    /// Manifest.
    pub(crate) manifest: Option<ObjectsManifest>,

    /// Terrain.
    pub(crate) terrain: TerrainSizeM,

    /// Prefab by id.
    pub(crate) prefab_by_id: HashMap<u64, PrefabEntry>,

    /// Has oversized.
    pub(crate) has_oversized: bool,

    /// Building by u16.
    pub(crate) building_by_u16: HashMap<u16, BuildingPrefabInfo>,

    /// Min importance zoom.
    pub(crate) min_importance_zoom: Option<f64>,

    /// Importance breakpoints.
    pub(crate) importance_breakpoints: Vec<f64>,

    /// Fence by u16.
    pub(crate) fence_by_u16: HashMap<u16, FencePrefabInfo>,

    /// Glyph by u16.
    pub(crate) glyph_by_u16: HashMap<u16, GlyphPrefabInfo>,

    /// Icon key to idx.
    pub(crate) icon_key_to_idx: HashMap<String, u16>,

    /// Cell ids.
    pub(crate) cell_ids: Option<HashSet<String>>,

    /// Chunks.
    pub(crate) chunks: HashMap<String, WorldChunk>,

    /// Building counts.
    pub(crate) building_counts: HashMap<String, u32>,

    /// Last used.
    pub(crate) last_used: HashMap<String, u64>,

    /// Inserted seq.
    pub(crate) inserted_seq: HashMap<String, u64>,

    /// Use tick.
    pub(crate) use_tick: u64,

    /// Insert counter.
    pub(crate) insert_counter: u64,

    /// Pinned ids.
    pub(crate) pinned_ids: Vec<String>,

    /// Pinned set.
    pub(crate) pinned_set: HashSet<String>,

    /// Pinned key.
    pub(crate) pinned_key: String,

    /// Inflight.
    pub(crate) inflight: HashSet<String>,

    /// Index.
    pub(crate) index: WorldSpatialIndex,

    /// Eviction log.
    pub(crate) eviction_log: Vec<String>,

    /// Residency events.
    pub(crate) residency_events: Vec<ResidencyEvent>,

    /// Chunks applied.
    pub(crate) chunks_applied: u64,

    /// Apply frames.
    pub(crate) apply_frames: u64,

    /// Max apply ms.
    pub(crate) max_apply_ms: f64,

    /// Frames over budget.
    pub(crate) frames_over_budget: u64,

    /// Apply budget ms last.
    pub(crate) apply_budget_ms_last: f64,

    /// Content epoch.
    pub(crate) content_epoch: u64,

    /// Glyph base key.
    pub(crate) glyph_base_key: u64,

    /// Strip key.
    pub(crate) strip_key: u64,

    /// Known empty.
    pub(crate) known_empty: HashSet<String>,

    /// Fetch failures.
    pub(crate) fetch_failures: HashMap<String, u8>,

    /// Buffers revision.
    pub(crate) buffers_revision: u64,

    /// Glyph recomposes.
    pub(crate) glyph_recomposes: u64,

    /// Fill recomposes.
    pub(crate) fill_recomposes: u64,

    /// Fill buf.
    pub(crate) fill_buf: Vec<f32>,

    /// Outline buf.
    pub(crate) outline_buf: Vec<f32>,

    /// Strip buf.
    pub(crate) strip_buf: Vec<f32>,

    /// Fence strip count.
    pub(crate) fence_strip_count: u32,

    /// Pier strip count.
    pub(crate) pier_strip_count: u32,

    /// Bridge rail count.
    pub(crate) bridge_rail_count: u32,

    /// Ingest frame start ms.
    pub(crate) ingest_frame_start_ms: Option<f64>,

    /// Deck zoom.
    pub(crate) deck_zoom: f64,

    /// Toggle trees.
    pub(crate) toggle_trees: bool,

    /// Toggle props.
    pub(crate) toggle_props: bool,

    /// Toggle buildings.
    pub(crate) toggle_buildings: bool,

    /// Toggle fences.
    pub(crate) toggle_fences: bool,

    /// Toggle airfield.
    pub(crate) toggle_airfield: bool,

    /// Airfield bbox.
    pub(crate) airfield_bbox: Option<Bbox>,

    /// Tree glyph buf.
    pub(crate) tree_glyph_buf: Vec<u8>,

    /// Prop glyph buf.
    pub(crate) prop_glyph_buf: Vec<u8>,

    /// Badge glyph buf.
    pub(crate) badge_glyph_buf: Vec<u8>,

    /// Tree want.
    pub(crate) tree_want: bool,

    /// Prop want.
    pub(crate) prop_want: bool,

    /// Badge want.
    pub(crate) badge_want: bool,

    /// Last viewport.
    pub(crate) last_viewport: Bbox,

    /// Draw ids.
    pub(crate) draw_ids: Vec<String>,

    /// Heatmap trees.
    pub(crate) heatmap_trees: bool,

    /// Exact tree count.
    pub(crate) exact_tree_count: u32,

    /// Glyph size floor zoom.
    pub(crate) glyph_size_floor_zoom: f64,
}

impl Default for WorldResidency {
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
            glyph_by_u16: HashMap::new(),
            icon_key_to_idx: HashMap::new(),
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
            glyph_base_key: 0,
            strip_key: 0,
            known_empty: HashSet::new(),
            fetch_failures: HashMap::new(),
            buffers_revision: 0,
            glyph_recomposes: 0,
            fill_recomposes: 0,
            fill_buf: Vec::new(),
            outline_buf: Vec::new(),
            strip_buf: Vec::new(),
            fence_strip_count: 0,
            pier_strip_count: 0,
            bridge_rail_count: 0,
            ingest_frame_start_ms: None,
            deck_zoom: -2.0,
            toggle_trees: true,
            toggle_props: false,
            toggle_buildings: true,
            toggle_fences: true,
            toggle_airfield: true,
            airfield_bbox: None,
            tree_glyph_buf: Vec::new(),
            prop_glyph_buf: Vec::new(),
            badge_glyph_buf: Vec::new(),
            tree_want: false,
            prop_want: false,
            badge_want: false,
            last_viewport: [0.0, 0.0, 0.0, 0.0],
            draw_ids: Vec::new(),
            heatmap_trees: false,
            exact_tree_count: 0,
            glyph_size_floor_zoom: 0.0,
        }
    }
}

impl WorldResidency {
    /// New.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
