//! Viewport-driven chunk residency — the Rust port of `chunkStore.ts` (the streaming half of the
//! world lane, for the wgpu path). **Class S** vs `createChunkStore`: for an identical viewport +
//! delivery sequence it requests the same chunk ids and evicts identically. The Deck
//! `chunkStore`/worker path is untouched — this is a separate type for the wgpu mount.
//!
//! Split API mirroring `runViewport` / `drainFrame` so a parity harness can drive this and the JS
//! oracle through one sequence:
//! - [`WorldResidency::set_viewport`] — pin/tick/evict, returns the missing chunk ids to fetch.
//! - [`WorldResidency::ingest_chunk_gz`] — parse + insert one delivered chunk (the `applyChunk`).
//! - [`WorldResidency::note_undelivered`] — cache a missing file as hydrated-empty (no refetch).
//! - [`WorldResidency::end_apply_frame`] — post-frame stats + evict + rebuild (the `drainFrame` tail).
//!
//! **Coordinates are emitted in WORLD meters**; the render engine subtracts its own
//! `scene::ANCHOR` when packing GPU instances (single anchor source of truth in the render crate).
//!
//! LRU exactness: cap = `max(LRU_MIN_CHUNKS, 3 × pinned)`, pinned never evicted, evict ascending
//! `last_used`. `use_tick` strictly increments on every touch, so `last_used` is unique per
//! resident chunk ⇒ a total order (the `inserted_seq` secondary key mirrors JS `Map` insertion
//! order but never actually breaks a tie).

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::airfield::{compute_airfield_bbox, is_airfield_structure_class, point_in_bbox};
use super::cartographic_strip::{
    compose_bridge_rail_strips, compose_fence_strip, compose_pier_strip, pack_cartographic_strips,
};
use super::chunk::{WorldChunk, parse_chunk};
use super::chunk_math::{
    Bbox, TerrainSizeM, chunk_ids_for_rect, chunk_ids_for_viewport, chunk_rect_for_bbox,
};
use super::classify::class_code;
use super::density_ladder::{
    density_grid_dims, exact_tree_count, heatmap_trees, pack_density_grid_r32, visible_tree_count,
};
use super::glyph_math::{
    BADGE_SIZE_MIN_PX, DEFAULT_BASE_SIZE_PX, GLYPH_SIZE_MIN_PX, badge_size_meters,
    deck_angle_for_rotation_deg, glyph_size_meters, hex_to_rgba, landmark_glyph_icon_key,
    pack_icon_instance, pack_rgba_u32, size_with_min_px,
};
use super::index::WorldSpatialIndex;
use super::lod_gates::{INSTANCE_BUDGET, class_visible};
use super::manifest::{ObjectsManifest, narrow_cells, parse_objects_manifest};
use super::obb::{
    BuildingPrefabInfo, FencePrefabInfo, building_prefab_lookup, fence_prefab_lookup, obb_corners,
};
use super::prefab::{PrefabEntry, build_prefab_maps, narrow_prefab_rows};
use super::store::{WorldError, bytes_to_json};

/// No extra draw margin — residency preload covers fetch; draw cull is strict visible rect (T-151.8).
/// Referenced by Class S tests / verify log; must stay 0.
pub const DRAW_CULL_MARGIN_M: f64 = 0.0;

/// Per-prefab glyph render resolved once at prefab load (tree/veg/prop/rockLarge/building).
#[derive(Clone, Debug)]
struct GlyphPrefabInfo {
    /// Index into the atlas UV table (`u16::MAX` = unknown key).
    glyph_idx: u16,
    size_m: f32,
    tint: u32,
    /// 0 = tree group (tree+vegetation), 1 = prop group (prop+rockLarge), 2 = building landmark.
    group: u8,
}

/// Per-frame ingest budget, ms (`chunkStore.ts` `APPLY_BUDGET_MS`).
pub const APPLY_BUDGET_MS: f64 = 4.0;
/// LRU floor (`chunkStore.ts` `LRU_MIN_CHUNKS`).
pub const LRU_MIN_CHUNKS: usize = 64;
/// Building-footprint LOD gate (`lodGates.ts` `BUILDING_FOOTPRINT_MIN_ZOOM`; manifest agrees).
pub const BUILDING_MIN_ZOOM: f64 = -2.5;

/// Building outline casing — near-black (spec L8; diverges from the Deck grey `STROKE`, logged).
const OUTLINE_COLOR: [u8; 4] = [30, 30, 34, 255];
/// Solid-dark default footprint fill (`buildingLayer.ts` `FILL_DEFAULT`).
const FILL_DEFAULT: [u8; 4] = [38, 38, 44, 184];

/// T-152.15 Q6 — bridge deck tint (warm stone), distinct from the gray building fill.
const BRIDGE_DECK_RGBA: [u8; 4] = [120, 116, 110, 215];
/// T-152.15 Q6 — dark casing rim drawn under the deck (a bridge reads as a bridge, not a building).
const BRIDGE_CASING_RGBA: [u8; 4] = [40, 40, 46, 220];
/// T-152.15 Q6 — casing widen (m) along the crossing (long) axis, past the deck ends.
const BRIDGE_CASING_MARGIN_M: f64 = 0.8;

/// `FILL_BY_CLASS[class] ?? FILL_DEFAULT` (`buildingLayer.ts:127-139`), verbatim.
#[must_use]
fn fill_color(class: &str) -> [u8; 4] {
    match class {
        "military" => [0x7a, 0x5c, 0x3d, 184],
        // T-152.15 Q6 — `bridge` is handled directly in `rebuild_buffers` (deck + casing), so it is
        // intentionally not in this table (one bridge codepath).
        "pier" | "dock" => [110, 95, 75, 190],
        "ruin" => [58, 56, 60, 110],
        "castle" => [70, 58, 48, 190],
        "lighthouse" => [235, 235, 235, 220],
        "container" => [60, 70, 90, 184],
        "tent" => [92, 82, 50, 184],
        "shed" | "garage" => [50, 50, 56, 184],
        _ => FILL_DEFAULT,
    }
}

#[inline]
fn building_visible(deck_zoom: f64) -> bool {
    deck_zoom >= BUILDING_MIN_ZOOM
}

fn norm(c: [u8; 4]) -> [f32; 4] {
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
        f32::from(c[3]) / 255.0,
    ]
}

/// T-173 P3 — disposition of one ingested chunk payload (see [`WorldResidency::ingest_chunk_gz`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestOutcome {
    /// Parsed + inserted with this many instances.
    Applied(u32),
    /// Well-formed chunk JSON with zero instances — stub kept, marked known-empty forever.
    ParsedEmpty,
    /// Payload did not match the chunk shape — counted toward the retry cap.
    ShapeMismatch,
}

/// T-173 P3 — failed deliveries per chunk before an undelivered stub is cached.
pub const FETCH_FAILURE_CAP: u8 = 3;

/// Multi-chunk residency + LRU + world spatial index + building/glyph GPU-buffer composer.
pub struct WorldResidency {
    manifest: Option<ObjectsManifest>,
    terrain: TerrainSizeM,
    prefab_by_id: HashMap<u64, PrefabEntry>,
    has_oversized: bool,
    /// Building footprint info keyed by `u16` — the effective key domain of the JS
    /// `buildingInfo.get(prefabIdx[k])` lookup (`prefabIdx` is a `Uint16Array`). Only prefab ids
    /// that are integers in `[0, 65536)` are inserted (JS finds a building iff `prefabId ===` the
    /// stored u16, i.e. `prefabId < 65536`).
    building_by_u16: HashMap<u16, BuildingPrefabInfo>,
    /// T-152.21 — smallest `importanceZoom` across resident building prefabs (`None` when none carry
    /// the override). Cheap outer guard: the badge lane may skip its whole loop when `deck_zoom` is
    /// below every override, preserving the non-landmark fast path.
    min_importance_zoom: Option<f64>,
    /// Fence prop half-extents keyed by prefab u16 id (T-152.4).
    fence_by_u16: HashMap<u16, FencePrefabInfo>,
    /// Tree/veg/prop/rockLarge glyph lookup keyed by prefab u16 id.
    glyph_by_u16: HashMap<u16, GlyphPrefabInfo>,
    /// Atlas iconKey → UV-table index (set by [`Self::set_glyph_key_map`]).
    icon_key_to_idx: HashMap<String, u16>,
    cell_ids: Option<HashSet<String>>,

    chunks: HashMap<String, WorldChunk>,
    building_counts: HashMap<String, u32>,
    last_used: HashMap<String, u64>,
    inserted_seq: HashMap<String, u64>,
    use_tick: u64,
    insert_counter: u64,
    pinned_ids: Vec<String>,
    pinned_set: HashSet<String>,
    pinned_key: String,
    inflight: HashSet<String>,
    index: WorldSpatialIndex,
    eviction_log: Vec<String>,

    chunks_applied: u64,
    apply_frames: u64,
    max_apply_ms: f64,
    frames_over_budget: u64,
    apply_budget_ms_last: f64,

    /// T-173 P2 — bumps whenever resident chunk *content* changes (insert / invalidate / evict),
    /// so the glyph memo below recomposes when a newly-ingested chunk enters the draw set even
    /// though the viewport rect is unchanged.
    content_epoch: u64,
    /// T-173 P2 — hash of the last glyph/strip/density compose inputs (draw-id set, zoom, content
    /// epoch, toggles, airfield/landmark state). `refresh_draw_set_and_glyphs` returns early on a
    /// match instead of re-packing up to 150 k instances every `set_viewport`.
    last_compose_key: u64,

    /// T-173 P3 — chunks that parsed cleanly to **zero instances**: their stubs are never
    /// evicted and never re-requested (the pre-T-173 host loop invalidated + refetched every
    /// legit-empty chunk on every settle, forever). `invalidate_chunk` clears the mark.
    known_empty: HashSet<String>,
    /// T-173 P3 — per-chunk failed-delivery count (HTTP error / gzip / shape mismatch). Below
    /// [`FETCH_FAILURE_CAP`] the id stays unresident so the next viewport pass retries; at cap an
    /// undelivered stub is cached (evictable → a later re-pin after eviction retries fresh).
    /// Cleared on successful apply and on every pin-key change.
    fetch_failures: HashMap<String, u8>,

    /// T-173 — monotonic output-buffer revision: bumps once per real recompose of any GPU-facing
    /// buffer family (fills/outlines/strips/glyphs/density). The host skips its clone+upload pass
    /// when the revision it last pushed is still current.
    buffers_revision: u64,
    /// T-173 — recompose counters (perf gates G-C/G-D): how many times the glyph/strip/density
    /// pack ran vs how many times the building fill/outline compose ran.
    glyph_recomposes: u64,
    fill_recomposes: u64,

    fill_buf: Vec<f32>,
    outline_buf: Vec<f32>,
    /// T-152.4 fence + pier thin strips (packed `[x,y,r,g,b,a]…` triangle list).
    strip_buf: Vec<f32>,
    /// T-152.15 — exact per-class strip instance counts (census gates G3/G5, decoupling G6).
    fence_strip_count: u32,
    pier_strip_count: u32,
    bridge_rail_count: u32,

    /// T-151.11.3 (audit B-04): ingest-frame start stamp — the ≤ APPLY_BUDGET_MS/frame policy
    /// lives HERE (pure; the wasm wrapper feeds `Date.now`), not in the JS loader.
    ingest_frame_start_ms: Option<f64>,

    /// Last viewport zoom (for glyph LOD + min-px clamp).
    deck_zoom: f64,
    /// User prefs (`worldLayerPrefs.classToggles`).
    toggle_trees: bool,
    toggle_props: bool,
    toggle_buildings: bool,
    /// T-152.4 cartographic fence/railing strips (default on).
    toggle_fences: bool,
    /// T-152.5 airfield apron + runway polish + hangar/tower icons (default on).
    toggle_airfield: bool,
    /// NW Everon airfield bbox from runway union + margin; set after roads load.
    airfield_bbox: Option<Bbox>,

    /// Packed 20 B icon instances (WORLD coords) — replace-not-accumulate.
    tree_glyph_buf: Vec<u8>,
    prop_glyph_buf: Vec<u8>,
    badge_glyph_buf: Vec<u8>,

    /// Last `set_viewport` strict bbox (no preload) — source for draw-set cull.
    last_viewport: Bbox,
    /// Strict visible ∩ pinned ∩ cells (sorted). Glyph/heatmap compose over this only.
    draw_ids: Vec<String>,
    /// Exact-count density ladder: heatmap rung active for trees.
    heatmap_trees: bool,
    /// Last exact tree+veg count over `draw_ids` (Class R).
    exact_tree_count: u32,
    /// R32Uint count grid (row-major cy×cx) for resident chunks.
    density_grid: Vec<u32>,
    density_grid_w: u32,
    density_grid_h: u32,
}

fn deinterleave(positions: &[f32], count: u32) -> (Vec<f32>, Vec<f32>) {
    let n = count as usize;
    let mut xs = Vec::with_capacity(n);
    let mut ys = Vec::with_capacity(n);
    for i in 0..n {
        xs.push(positions[2 * i]);
        ys.push(positions[2 * i + 1]);
    }
    (xs, ys)
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
            chunks_applied: 0,
            apply_frames: 0,
            max_apply_ms: 0.0,
            frames_over_budget: 0,
            apply_budget_ms_last: 0.0,
            content_epoch: 0,
            last_compose_key: 0,
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
            last_viewport: [0.0, 0.0, 0.0, 0.0],
            draw_ids: Vec::new(),
            heatmap_trees: false,
            exact_tree_count: 0,
            density_grid: Vec::new(),
            density_grid_w: 0,
            density_grid_h: 0,
        }
    }
}

impl WorldResidency {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register atlas icon keys in UV-table order (must match `upload_glyph_atlas` UV order).
    /// Rebuilds the glyph prefab lookup when prefabs are already loaded.
    pub fn set_glyph_key_map(&mut self, keys: &[String]) {
        self.icon_key_to_idx.clear();
        for (i, k) in keys.iter().enumerate() {
            if i < usize::from(u16::MAX) {
                self.icon_key_to_idx.insert(k.clone(), i as u16);
            }
        }
        self.rebuild_glyph_lookup_from_prefabs();
        // The atlas key→idx map is not part of the compose memo key (T-173 P2) — force a recompose
        // so glyphs pick up the freshly-registered UV indices.
        self.last_compose_key = 0;
        self.refresh_draw_set_and_glyphs();
    }

    /// User layer toggles (mirrors `worldLayerPrefs.classToggles` trees/props/buildings).
    pub fn set_glyph_toggles(&mut self, trees: bool, props: bool, buildings: bool) {
        if self.toggle_trees == trees
            && self.toggle_props == props
            && self.toggle_buildings == buildings
        {
            return;
        }
        self.toggle_trees = trees;
        self.toggle_props = props;
        self.toggle_buildings = buildings;
        // T-151.11.3 (P-04): the buildings toggle hides the WHOLE lane (fills + outlines +
        // badges — Deck semantics), so the footprint buffers must rebuild too, not just glyphs.
        self.rebuild_buffers();
    }

    /// T-152.4 — cartographic fence/railing strip toggle (`worldLayerPrefs.fences`).
    pub fn set_fences_toggle(&mut self, fences: bool) {
        if self.toggle_fences == fences {
            return;
        }
        self.toggle_fences = fences;
        self.rebuild_strip_buffers();
    }

    /// T-152.5 — airfield apron / runway polish / hangar-tower icon toggle.
    pub fn set_airfield_toggle(&mut self, on: bool) {
        if self.toggle_airfield == on {
            return;
        }
        self.toggle_airfield = on;
        self.rebuild_glyph_buffers();
    }

    /// Set airfield bbox from runway segments (call after roads load).
    pub fn set_airfield_bbox_from_runways(&mut self, runways: &[super::roads::RoadSegment]) {
        self.airfield_bbox = compute_airfield_bbox(runways);
        self.rebuild_glyph_buffers();
    }

    /// Whether airfield-specific icons draw (user toggle ∧ bbox known).
    #[must_use]
    pub fn airfield_visible(&self) -> bool {
        self.toggle_airfield && self.airfield_bbox.is_some()
    }

    #[must_use]
    pub fn airfield_bbox(&self) -> Option<Bbox> {
        self.airfield_bbox
    }

    /// Whether the fence strip lane should draw: Fences toggle ∧ fence LOD gate (T-152.15 —
    /// dedicated `fence` class at z ≥ 1.5, decoupled from piers).
    #[must_use]
    pub fn fences_visible(&self) -> bool {
        self.toggle_fences && class_visible("fence", self.deck_zoom)
    }

    /// Whether the pier/dock quay lane should draw: Buildings toggle ∧ pier LOD gate (T-152.15 —
    /// dedicated `pier` class at z ≥ −1.0, decoupled from the Fences toggle).
    #[must_use]
    pub fn piers_visible(&self) -> bool {
        self.toggle_buildings && class_visible("pier", self.deck_zoom)
    }

    /// Whether the building fill/outline lanes should draw: user toggle ∧ zoom gate
    /// (T-151.11.3 / P-04). The loader passes this as the upload `visible` flag so a toggle-off
    /// removes the lanes (bypassing the empty+visible sticky anti-wipe rule, which only guards
    /// mid-hydration wipes).
    #[must_use]
    pub fn buildings_visible(&self) -> bool {
        self.toggle_buildings && building_visible(self.deck_zoom)
    }

    /// Whether the shared strip render lane (fences + piers + bridge rails) may draw at all —
    /// stable on toggle+zoom only (fences OR piers OR buildings). The loader passes this as the
    /// upload `visible` flag; keeping it independent of buffer contents preserves the
    /// empty+visible mid-hydration anti-wipe guard while the three sub-lanes gate independently.
    #[must_use]
    pub fn strips_visible(&self) -> bool {
        self.fences_visible() || self.piers_visible() || self.buildings_visible()
    }

    fn rebuild_glyph_lookup_from_prefabs(&mut self) {
        self.glyph_by_u16.clear();
        if self.icon_key_to_idx.is_empty() {
            return;
        }
        for entry in self.prefab_by_id.values() {
            let pid = entry.row.prefab_id;
            if !(0.0..65536.0).contains(&pid) || pid.fract() != 0.0 {
                continue;
            }
            let Some(icon_key) = entry.row.icon_key.as_deref() else {
                continue;
            };
            let Some(&glyph_idx) = self.icon_key_to_idx.get(icon_key) else {
                continue;
            };
            let code = entry.code;
            let base = entry.row.base_size_px.unwrap_or(DEFAULT_BASE_SIZE_PX);
            let tree_code = class_code("tree");
            let veg_code = class_code("vegetation");
            let prop_code = class_code("prop");
            let rock_code = class_code("rockLarge");
            let building_code = class_code("building");
            let (group, size_m, tint) = if code == tree_code || code == veg_code {
                (
                    0u8,
                    glyph_size_meters(base, entry.row.height_m) as f32,
                    pack_rgba_u32(hex_to_rgba(entry.row.default_color.as_deref())),
                )
            } else if code == prop_code || code == rock_code {
                (
                    1u8,
                    glyph_size_meters(base, entry.row.height_m) as f32,
                    pack_rgba_u32(hex_to_rgba(entry.row.default_color.as_deref())),
                )
            } else if code == building_code {
                (
                    2u8,
                    badge_size_meters() as f32,
                    pack_rgba_u32([255, 255, 255, 255]),
                )
            } else {
                continue;
            };
            self.glyph_by_u16.insert(
                pid as u16,
                GlyphPrefabInfo {
                    glyph_idx,
                    size_m,
                    tint,
                    group,
                },
            );
        }
    }

    /// Parse the terrain manifest: the `objects` block (chunk size + object-export gate) and the
    /// top-level `worldBounds` (terrain extent for the chunk-id math).
    ///
    /// # Errors
    /// [`WorldError::Json`] on invalid JSON; [`WorldError::Manifest`] when the object paths are
    /// absent.
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

    /// Load + narrow `prefabs.json.gz`: the class table (`has_oversized`) and the u16-keyed
    /// building footprint lookup. Returns the prefab count.
    ///
    /// # Errors
    /// [`WorldError::Gzip`]/[`WorldError::Json`] on a bad payload.
    pub fn load_prefabs_gz(&mut self, bytes: &[u8]) -> Result<usize, WorldError> {
        let raw = bytes_to_json(bytes)?;
        let (by_id, has_oversized) = build_prefab_maps(narrow_prefab_rows(&raw));
        self.prefab_by_id = by_id;
        self.has_oversized = has_oversized;
        self.building_by_u16.clear();
        for (bits, info) in building_prefab_lookup(&raw) {
            let pid = f64::from_bits(bits);
            if (0.0..65536.0).contains(&pid) && pid.fract() == 0.0 {
                self.building_by_u16.insert(pid as u16, info);
            }
        }
        // T-152.21 — cache the smallest importanceZoom so the badge lane's outer guard is O(1).
        self.min_importance_zoom = self
            .building_by_u16
            .values()
            .filter_map(|b| b.importance_zoom)
            .fold(None, |acc, v| Some(acc.map_or(v, |a: f64| a.min(v))));
        self.fence_by_u16 = fence_prefab_lookup(&raw);
        self.rebuild_glyph_lookup_from_prefabs();
        Ok(self.prefab_by_id.len())
    }

    /// Load the chunk-index (`objects/chunks/manifest.json`) `cells[]` → the existing-chunk id set
    /// the viewport request is intersected with. Returns the cell count.
    ///
    /// # Errors
    /// [`WorldError::Json`] on invalid JSON.
    pub fn load_chunk_index_json(&mut self, json: &str) -> Result<usize, WorldError> {
        let raw: Value = serde_json::from_str(json).map_err(|e| WorldError::Json(e.to_string()))?;
        let cells = narrow_cells(&raw).unwrap_or_default();
        let set: HashSet<String> = cells.into_iter().map(|c| c.id).collect();
        let n = set.len();
        self.cell_ids = Some(set);
        Ok(n)
    }

    /// `runViewport(bbox, deckZoom)` — pin the viewport chunk set, re-touch resident members,
    /// evict, and return the **missing** ids to fetch (already marked in-flight so an overlapping
    /// viewport won't re-request them). Returns empty when below the building band or when the
    /// chunk set is unchanged.
    pub fn set_viewport(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        deck_zoom: f64,
    ) -> Vec<String> {
        let prev_zoom = self.deck_zoom;
        let zoom_changed = (prev_zoom - deck_zoom).abs() > f64::EPSILON;
        self.deck_zoom = deck_zoom;
        self.last_viewport = [min_x, min_y, max_x, max_y];
        // T-152.21 — keep world chunks resident below the footprint gate when a landmark's
        // importanceZoom override could still draw its badge (deck_zoom ≥ min override). Fills
        // themselves stay gated at BUILDING_MIN_ZOOM inside `rebuild_buffers`; only the badge lane
        // draws in the [min_importance, BUILDING_MIN_ZOOM) band.
        let world_want =
            building_visible(deck_zoom) || self.min_importance_zoom.is_some_and(|m| deck_zoom >= m);
        if !world_want {
            if !self.pinned_ids.is_empty() {
                self.pinned_ids.clear();
                self.pinned_set.clear();
                self.pinned_key.clear();
                self.rebuild_buffers();
            } else {
                // Glyph LOD / draw-set may still change with zoom or viewport.
                let _ = zoom_changed;
                self.refresh_draw_set_and_glyphs();
            }
            return Vec::new();
        }
        let chunk_size_m = match &self.manifest {
            Some(m) => m.chunk_size_m,
            None => return Vec::new(),
        };
        let extra_ring = i64::from(self.has_oversized);
        let mut ids = chunk_ids_for_viewport(
            [min_x, min_y, max_x, max_y],
            self.terrain,
            chunk_size_m,
            extra_ring,
        );
        if let Some(cells) = &self.cell_ids {
            ids.retain(|id| cells.contains(id));
        }
        let key = ids.join(",");
        if key == self.pinned_key {
            // Zoom change with same chunks still needs glyph recompose (LOD / min-px).
            // Draw-set always refreshes from last_viewport (strict rect may differ from pin).
            // T-152.21: a pure zoom change can also flip landmark fill de-emphasis (badge band /
            // importance boundary) — recompose fills when that band changed (rebuild_buffers also
            // refreshes the draw set + glyphs), else just refresh glyphs.
            if self.fill_band_changed(prev_zoom, deck_zoom) {
                self.rebuild_buffers();
            } else {
                self.refresh_draw_set_and_glyphs();
            }
            // T-151.4.1: same pin key usually means "nothing new". But an aborted fetch can leave
            // the pin unsettled with empty inflight — re-request those undelivered ids.
            if self.pin_settled() || !self.inflight.is_empty() {
                return Vec::new();
            }
            let missing: Vec<String> = self
                .pinned_ids
                .iter()
                .filter(|id| !self.chunks.contains_key(*id))
                .cloned()
                .collect();
            for id in &missing {
                self.inflight.insert(id.clone());
            }
            return missing;
        }
        self.pinned_ids = ids.clone();
        self.pinned_set = ids.iter().cloned().collect();
        self.pinned_key = key;
        // New pin epoch → failed-delivery counters reset (a transient outage during one camera
        // hold must not permanently cap chunks the operator pans back to; T-173 P3).
        self.fetch_failures.clear();
        // Re-touch resident pinned ids in row-major order (bumps last_used, matching runViewport).
        for id in &ids {
            if self.chunks.contains_key(id) {
                self.use_tick += 1;
                self.last_used.insert(id.clone(), self.use_tick);
            }
        }
        let missing: Vec<String> = ids
            .iter()
            .filter(|id| !self.chunks.contains_key(*id) && !self.inflight.contains(*id))
            .cloned()
            .collect();
        for id in &missing {
            self.inflight.insert(id.clone());
        }
        self.evict();
        self.rebuild_buffers();
        missing
    }

    /// `applyChunk` — parse one delivered `objects/chunks/{id}.json.gz` and insert it into the
    /// residency + spatial index. Returns the T-173 disposition. (The caller runs the per-frame
    /// budget loop; call [`Self::end_apply_frame`] after a frame that applied ≥1 chunk.)
    ///
    /// Policy (T-173 P3 — replaces the host's invalidate/refetch paranoia loop):
    /// - well-formed, `count > 0` → [`IngestOutcome::Applied`]; failure counter + empty mark clear
    /// - well-formed, zero instances → stub + permanent `known_empty` mark ([`IngestOutcome::ParsedEmpty`])
    /// - payload not chunk-shaped → counted toward [`FETCH_FAILURE_CAP`] ([`IngestOutcome::ShapeMismatch`])
    ///
    /// # Errors
    /// [`WorldError::Gzip`]/[`WorldError::Json`] on a bad payload — the caller routes those to
    /// [`Self::note_fetch_failure`] like an HTTP failure.
    pub fn ingest_chunk_gz(&mut self, id: &str, bytes: &[u8]) -> Result<IngestOutcome, WorldError> {
        let raw = bytes_to_json(bytes)?;
        match parse_chunk(id, &raw, &self.prefab_by_id) {
            Some(chunk) if chunk.count > 0 => {
                let count = chunk.count;
                self.insert_chunk(id, chunk);
                self.known_empty.remove(id);
                self.fetch_failures.remove(id);
                Ok(IngestOutcome::Applied(count))
            }
            Some(chunk) => {
                self.insert_chunk(id, chunk);
                self.known_empty.insert(id.to_string());
                self.fetch_failures.remove(id);
                Ok(IngestOutcome::ParsedEmpty)
            }
            None => {
                self.note_fetch_failure(id);
                Ok(IngestOutcome::ShapeMismatch)
            }
        }
    }

    /// T-173 P3 — record a failed delivery (HTTP error, gzip/json corruption, shape mismatch).
    /// Below [`FETCH_FAILURE_CAP`] the id is released from inflight so the next viewport pass
    /// retries it; at the cap an undelivered stub is cached (the id stops thrashing; a later
    /// eviction + re-pin retries fresh because pin-key changes clear the counters).
    pub fn note_fetch_failure(&mut self, id: &str) {
        let n = self.fetch_failures.entry(id.to_string()).or_insert(0);
        *n += 1;
        if *n >= FETCH_FAILURE_CAP {
            self.fetch_failures.remove(id);
            self.note_undelivered(id);
        } else {
            self.inflight.remove(id);
        }
    }

    /// Cache a requested-but-undelivered chunk (missing/empty file) as hydrated-empty so it is
    /// never re-requested (`requestMissing`'s `applied.set(id, [])`).
    pub fn note_undelivered(&mut self, id: &str) {
        self.insert_chunk(
            id,
            WorldChunk {
                id: id.to_string(),
                ..Default::default()
            },
        );
    }

    fn insert_chunk(&mut self, id: &str, chunk: WorldChunk) {
        let building_code = class_code("building");
        let building_count = chunk.rows_by_class.get(&building_code).map_or(0, |rows| {
            rows.iter()
                .filter(|&&r| {
                    self.building_by_u16
                        .contains_key(&chunk.prefab_idx[r as usize])
                })
                .count() as u32
        });
        let (xs, ys) = deinterleave(&chunk.positions, chunk.count);
        self.index.insert_chunk(id, &xs, &ys, &chunk.cls_codes);
        self.building_counts.insert(id.to_string(), building_count);
        self.chunks.insert(id.to_string(), chunk);
        self.use_tick += 1;
        self.last_used.insert(id.to_string(), self.use_tick);
        self.insert_counter += 1;
        self.inserted_seq
            .insert(id.to_string(), self.insert_counter);
        self.inflight.remove(id);
        self.chunks_applied += 1;
        self.content_epoch += 1;
    }

    /// T-151.11.3 (B-04): start an ingest frame at `now_ms`. The per-frame budget policy
    /// (`APPLY_BUDGET_MS`) is owned by this type; callers loop
    /// `while !ingest_budget_exhausted_at(now)` and close with [`Self::end_ingest_frame_at`].
    pub fn begin_ingest_frame_at(&mut self, now_ms: f64) {
        self.ingest_frame_start_ms = Some(now_ms);
    }

    /// True once the current ingest frame has consumed the apply budget. `false` when no frame
    /// is open (callers may ingest at least one chunk per frame regardless — the JS loop shape).
    #[must_use]
    pub fn ingest_budget_exhausted_at(&self, now_ms: f64) -> bool {
        match self.ingest_frame_start_ms {
            Some(start) => now_ms - start >= APPLY_BUDGET_MS,
            None => false,
        }
    }

    /// Close the ingest frame at `now_ms`: records stats + evicts + rebuilds via
    /// [`Self::end_apply_frame`]. No-op when no frame is open.
    pub fn end_ingest_frame_at(&mut self, now_ms: f64) {
        if let Some(start) = self.ingest_frame_start_ms.take() {
            self.end_apply_frame(now_ms - start);
        }
    }

    /// `drainFrame` tail — record the frame's apply stats, then evict + rebuild once. `elapsed_ms`
    /// is the wall time the caller measured for this frame's ingest loop.
    pub fn end_apply_frame(&mut self, elapsed_ms: f64) {
        self.apply_frames += 1;
        if elapsed_ms > self.max_apply_ms {
            self.max_apply_ms = elapsed_ms;
        }
        if elapsed_ms > APPLY_BUDGET_MS {
            self.frames_over_budget += 1;
        }
        self.apply_budget_ms_last = elapsed_ms;
        self.evict();
        self.rebuild_buffers();
    }

    /// `evictBeyondCap` — cap = `max(64, 3 × pinned)`; drop non-pinned chunks ascending `last_used`
    /// (unique ⇒ total order; `inserted_seq` mirrors JS Map order as a defensive tiebreak).
    fn evict(&mut self) {
        let cap = LRU_MIN_CHUNKS.max(3 * self.pinned_ids.len());
        if self.chunks.len() <= cap {
            return;
        }
        // T-173 P3 — known-empty stubs are never evicted: they hold no instance data (≈free) and
        // keeping them resident is what guarantees a legit-empty chunk is fetched at most once.
        let mut candidates: Vec<String> = self
            .chunks
            .keys()
            .filter(|id| !self.pinned_set.contains(*id) && !self.known_empty.contains(*id))
            .cloned()
            .collect();
        candidates.sort_by(|a, b| {
            let la = self.last_used.get(a).copied().unwrap_or(0);
            let lb = self.last_used.get(b).copied().unwrap_or(0);
            la.cmp(&lb).then_with(|| {
                let sa = self.inserted_seq.get(a).copied().unwrap_or(0);
                let sb = self.inserted_seq.get(b).copied().unwrap_or(0);
                sa.cmp(&sb)
            })
        });
        for id in candidates {
            if self.chunks.len() <= cap {
                break;
            }
            self.chunks.remove(&id);
            self.last_used.remove(&id);
            self.inserted_seq.remove(&id);
            self.building_counts.remove(&id);
            self.index.remove_chunk(&id);
            self.eviction_log.push(id);
            self.content_epoch += 1;
        }
    }

    /// T-152.21 — does a landmark's early badge glyph actually draw at the current zoom? True only
    /// below the badge band (`buildingBadge` class gate off) when the prefab's `importanceZoom`
    /// override fires (`deck_zoom >= importanceZoom`) **and** the atlas carries its landmark glyph.
    /// State-conditioned handoff (G4): a missing glyph → `false` → the bright class fill is kept, so
    /// a landmark is never left both rectangle-dimmed and glyph-less. At/above the badge band the
    /// class gate already composes the badge over the normal fill, so no de-emphasis applies.
    /// (The only bright-filled classes this de-emphasizes — lighthouse/castle/military — are not
    /// airfield-gated; `tower`/`hangar` already use the neutral default fill, so the airfield badge
    /// gate can't strand a de-emphasized-but-glyphless landmark.)
    fn early_landmark_glyph_active(&self, cls: &str, importance_zoom: Option<f64>) -> bool {
        let z = self.deck_zoom;
        if class_visible("buildingBadge", z) {
            return false;
        }
        let Some(iz) = importance_zoom else {
            return false;
        };
        if z < iz {
            return false;
        }
        landmark_glyph_icon_key(cls)
            .and_then(|k| self.icon_key_to_idx.get(k))
            .is_some()
    }

    /// T-152.21 — could a pure zoom change (`prev` → `next`) flip any landmark's fill de-emphasis,
    /// so the fill buffer must recompose (not just the glyph LOD)? The state flips only when the
    /// `buildingBadge` class gate toggles or the zoom crosses the importanceZoom boundary. NB: this
    /// assumes a single importanceZoom magnitude across resident prefabs (tracked as
    /// `min_importance_zoom`, today −4 everywhere); adding distinct values would need the full set.
    fn fill_band_changed(&self, prev: f64, next: f64) -> bool {
        // Footprint gate: fills/outlines appear/vanish at BUILDING_MIN_ZOOM (landmark badges can
        // persist below it — see `rebuild_buffers` + `set_viewport` residency guard).
        if building_visible(prev) != building_visible(next) {
            return true;
        }
        if class_visible("buildingBadge", prev) != class_visible("buildingBadge", next) {
            return true;
        }
        self.min_importance_zoom
            .is_some_and(|m| (prev >= m) != (next >= m))
    }

    /// Recompose the building fill + outline GPU buffers from the pinned chunks, in **string-sorted
    /// id order** (matching the JS composite `[...pinned].sort()`).
    fn rebuild_buffers(&mut self) {
        self.fill_recomposes += 1;
        // T-151.11.3 (P-04): toggle off ⇒ compose nothing (Deck hid the whole building lane).
        // T-152.21: below the footprint gate (BUILDING_MIN_ZOOM) draw no rectangles either, but
        // still refresh glyphs so importanceZoom landmark badges persist in that coarse band
        // ("landmark badges persist, ordinary buildings gone").
        if !self.toggle_buildings || !building_visible(self.deck_zoom) {
            self.fill_buf.clear();
            self.outline_buf.clear();
            self.rebuild_strip_buffers();
            self.refresh_draw_set_and_glyphs();
            return;
        }
        let building_code = class_code("building");
        let outline_norm = norm(OUTLINE_COLOR);
        let mut fill: Vec<f32> = Vec::new();
        let mut outline: Vec<f32> = Vec::new();
        let mut ids = self.pinned_ids.clone();
        ids.sort();
        for id in &ids {
            let Some(chunk) = self.chunks.get(id) else {
                continue;
            };
            let Some(rows) = chunk.rows_by_class.get(&building_code) else {
                continue;
            };
            for &r in rows {
                let r = r as usize;
                let Some(info) = self.building_by_u16.get(&chunk.prefab_idx[r]) else {
                    continue;
                };
                let x = f64::from(chunk.positions[2 * r]);
                let y = f64::from(chunk.positions[2 * r + 1]);
                let rot = f64::from(chunk.rotations[r]);
                let cls = info.building_class.as_str();
                // T-152.4/T-152.15: pier/dock render as thin quay strips (rebuild_strip_buffers),
                // never a fat square — skip the fill.
                if cls == "pier" || cls == "dock" {
                    continue;
                }
                // Fill instance (WORLD coords): [x, y, hx, hy, cos, sin, r,g,b,a]. `(cos,sin)` use
                // the same `rad = deg·PI/180` as `obb_corners`, so fill and outline coincide.
                let rad = (rot * std::f64::consts::PI) / 180.0;
                let cos = rad.cos() as f32;
                let sin = rad.sin() as f32;
                if cls == "bridge" {
                    // T-152.15 Q6 — dark casing rim widened along the crossing (long) axis, pushed
                    // FIRST, then the warm deck on top → reads as a bridge, not a gray building.
                    let (chx, chy) = if info.half_x >= info.half_y {
                        (info.half_x + BRIDGE_CASING_MARGIN_M, info.half_y)
                    } else {
                        (info.half_x, info.half_y + BRIDGE_CASING_MARGIN_M)
                    };
                    let casing = norm(BRIDGE_CASING_RGBA);
                    fill.extend_from_slice(&[
                        x as f32, y as f32, chx as f32, chy as f32, cos, sin, casing[0], casing[1],
                        casing[2], casing[3],
                    ]);
                    let deck = norm(BRIDGE_DECK_RGBA);
                    fill.extend_from_slice(&[
                        x as f32,
                        y as f32,
                        info.half_x as f32,
                        info.half_y as f32,
                        cos,
                        sin,
                        deck[0],
                        deck[1],
                        deck[2],
                        deck[3],
                    ]);
                } else {
                    // T-152.21 (G4): when the early landmark glyph draws (importanceZoom override
                    // below the badge band), hand the coarse-zoom face to the glyph — drop the bright
                    // class tint to the neutral footprint fill so the icon isn't shouted over by a
                    // white/tinted rectangle. Glyph unavailable ⇒ predicate false ⇒ bright fill kept.
                    let fill_rgba = if self.early_landmark_glyph_active(cls, info.importance_zoom) {
                        FILL_DEFAULT
                    } else {
                        fill_color(cls)
                    };
                    let c = norm(fill_rgba);
                    fill.extend_from_slice(&[
                        x as f32,
                        y as f32,
                        info.half_x as f32,
                        info.half_y as f32,
                        cos,
                        sin,
                        c[0],
                        c[1],
                        c[2],
                        c[3],
                    ]);
                }
                // Outline: closed LineList ring (world coords), 8 vertices, near-black.
                let ring = obb_corners(x, y, info.half_x, info.half_y, rot);
                for e in 0..4 {
                    let a = ring[e];
                    let b = ring[(e + 1) % 4];
                    outline.extend_from_slice(&[
                        a[0] as f32,
                        a[1] as f32,
                        outline_norm[0],
                        outline_norm[1],
                        outline_norm[2],
                        outline_norm[3],
                    ]);
                    outline.extend_from_slice(&[
                        b[0] as f32,
                        b[1] as f32,
                        outline_norm[0],
                        outline_norm[1],
                        outline_norm[2],
                        outline_norm[3],
                    ]);
                }
            }
        }
        self.fill_buf = fill;
        self.outline_buf = outline;
        self.rebuild_strip_buffers();
        self.refresh_draw_set_and_glyphs();
    }

    /// T-152.15 — compose pier quays, bridge rails, and fence strips over pinned chunks. Each lane
    /// gates independently (piers ← Buildings toggle + pier LOD; rails ← Buildings toggle; fences ←
    /// Fences toggle + fence LOD), with an exact instance count (gates G3/G5/G6). The three sub-vecs
    /// concatenate into one packed `strip_buf` → one render lane (WorldFences); order pier → rail →
    /// fence is cosmetic (piers/rails under fences).
    fn rebuild_strip_buffers(&mut self) {
        self.strip_buf.clear();
        self.fence_strip_count = 0;
        self.pier_strip_count = 0;
        self.bridge_rail_count = 0;
        let prop_code = class_code("prop");
        let building_code = class_code("building");
        let z = self.deck_zoom;
        let mut ids = self.pinned_ids.clone();
        ids.sort();
        let mut pier_v: Vec<crate::geometry::polyline_strip::StripVertex> = Vec::new();
        let mut rail_v: Vec<crate::geometry::polyline_strip::StripVertex> = Vec::new();
        let mut fence_v: Vec<crate::geometry::polyline_strip::StripVertex> = Vec::new();

        // Building-row lane: pier/dock quays (pier gate) + bridge rails (building gate). Both keyed
        // to the Buildings toggle only — never the Fences toggle.
        let piers_on = self.piers_visible();
        let rails_on = self.buildings_visible();
        if piers_on || rails_on {
            for id in &ids {
                let Some(chunk) = self.chunks.get(id) else {
                    continue;
                };
                let Some(rows) = chunk.rows_by_class.get(&building_code) else {
                    continue;
                };
                for &r in rows {
                    let r = r as usize;
                    let Some(info) = self.building_by_u16.get(&chunk.prefab_idx[r]) else {
                        continue;
                    };
                    let cls = info.building_class.as_str();
                    let x = f64::from(chunk.positions[2 * r]);
                    let y = f64::from(chunk.positions[2 * r + 1]);
                    let rot = f64::from(chunk.rotations[r]);
                    if piers_on && (cls == "pier" || cls == "dock") {
                        // T-152.15 L3 — every pier/dock emits exactly one quay strip.
                        pier_v.extend(compose_pier_strip(
                            x,
                            y,
                            info.half_x,
                            info.half_y,
                            rot,
                            fill_color(cls),
                            z,
                        ));
                        self.pier_strip_count += 1;
                    } else if rails_on && cls == "bridge" {
                        // T-152.15 Path A — 2 synthetic deck-edge rails per bridge (gate G5).
                        rail_v.extend(compose_bridge_rail_strips(
                            x,
                            y,
                            info.half_x,
                            info.half_y,
                            rot,
                            z,
                        ));
                        self.bridge_rail_count += 2;
                    }
                }
            }
        }

        // Fence prop strips — Fences toggle + fence LOD gate only (decoupled from piers/rails).
        if self.fences_visible() {
            for id in &ids {
                let Some(chunk) = self.chunks.get(id) else {
                    continue;
                };
                let Some(rows) = chunk.rows_by_class.get(&prop_code) else {
                    continue;
                };
                for &r in rows {
                    let r = r as usize;
                    let Some(finfo) = self.fence_by_u16.get(&chunk.prefab_idx[r]) else {
                        continue;
                    };
                    let x = f64::from(chunk.positions[2 * r]);
                    let y = f64::from(chunk.positions[2 * r + 1]);
                    let rot = f64::from(chunk.rotations[r]);
                    fence_v.extend(compose_fence_strip(
                        x,
                        y,
                        finfo.half_x,
                        finfo.half_y,
                        rot,
                        z,
                    ));
                    self.fence_strip_count += 1;
                }
            }
        }

        let mut acc = pier_v;
        acc.extend(rail_v);
        acc.extend(fence_v);
        self.strip_buf = pack_cartographic_strips(&acc);
    }

    /// T-173 P2 — hash of every input the glyph/strip/density pack reads, so
    /// `refresh_draw_set_and_glyphs` can skip an identical recompose. `draw_ids` must already be
    /// set (sorted) before this is called. Zoom is hashed by bits (min-px sizing is continuous),
    /// so any real camera move that changes glyph sizes still invalidates the memo.
    fn compose_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.draw_ids.hash(&mut h);
        self.deck_zoom.to_bits().hash(&mut h);
        self.content_epoch.hash(&mut h);
        self.toggle_trees.hash(&mut h);
        self.toggle_props.hash(&mut h);
        self.toggle_buildings.hash(&mut h);
        self.toggle_fences.hash(&mut h);
        self.toggle_airfield.hash(&mut h);
        // bbox / importance state changes the badge + airfield-icon set.
        self.airfield_bbox.map(|b| b.map(f64::to_bits)).hash(&mut h);
        self.min_importance_zoom.map(f64::to_bits).hash(&mut h);
        h.finish()
    }

    /// Strict visible rect → chunk ids ∩ pinned ∩ cells (Class S draw-set). No preload expand.
    /// [`DRAW_CULL_MARGIN_M`] is locked at 0 — do not expand `strict_bbox` before this call.
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

    /// Recompute `draw_ids` + density ladder + glyph buffers from `last_viewport`.
    fn refresh_draw_set_and_glyphs(&mut self) {
        self.draw_ids = self.draw_chunk_ids(self.last_viewport);
        // T-173 P2 — memo: the glyph/strip/density pack is a pure function of the draw-id set,
        // zoom (min-px sizing is continuous in zoom), resident-content epoch, the layer toggles,
        // and the airfield/landmark state. Recompute the key; a match means the last-composed
        // buffers are still correct — skip the ≤150 k-instance re-pack and don't bump the revision.
        let key = self.compose_key();
        if key == self.last_compose_key {
            return;
        }
        self.last_compose_key = key;
        let chunk_size_m = self
            .manifest
            .as_ref()
            .map(|m| m.chunk_size_m)
            .unwrap_or(512.0);
        let (gw, gh) = density_grid_dims(self.terrain.width, self.terrain.height, chunk_size_m);
        self.density_grid_w = gw;
        self.density_grid_h = gh;
        if gw > 0 && gh > 0 && self.terrain.width > 0.0 {
            self.density_grid = pack_density_grid_r32(&self.chunks, gw, gh);
        } else {
            self.density_grid.clear();
        }
        // Whole-chunk exact stays the stats/density-texel surface (the R32 grid is per-chunk).
        let exact = exact_tree_count(&self.chunks, &self.draw_ids);
        self.exact_tree_count = exact as u32;
        // T-152.14: the heatmap-vs-glyph swap counts viewport-VISIBLE trees (area-fraction, audit
        // A2) with hysteresis `[0.85×budget, budget]` — a chunk barely on-screen no longer clears
        // every glyph, and the rung does not flicker on pan. Enter above budget; stay resident in
        // the heatmap rung until the visible count drops below 0.85× (127_500 at the locked budget).
        let visible = visible_tree_count(
            &self.chunks,
            &self.draw_ids,
            self.last_viewport,
            chunk_size_m,
        );
        let reenter = INSTANCE_BUDGET * 85 / 100;
        self.heatmap_trees = if self.heatmap_trees {
            visible >= reenter
        } else {
            heatmap_trees(visible)
        };
        self.rebuild_strip_buffers();
        self.rebuild_glyph_buffers();
        self.glyph_recomposes += 1;
        self.buffers_revision += 1;
    }

    /// Compose tree / prop / badge glyph instance buffers from **draw_ids** only.
    /// Tree ladder: when `heatmap_trees`, tree glyphs are cleared (heatmap lane owns the rung).
    fn rebuild_glyph_buffers(&mut self) {
        self.tree_glyph_buf.clear();
        self.prop_glyph_buf.clear();
        self.badge_glyph_buf.clear();

        let z = self.deck_zoom;
        let tree_want =
            self.toggle_trees && (class_visible("tree", z) || class_visible("vegetation", z));
        let prop_want =
            self.toggle_props && (class_visible("prop", z) || class_visible("rockLarge", z));
        // T-152.21 — landmarks with an importanceZoom override draw their badge below the class
        // gate (deck_zoom ≥ importanceZoom). `badge_gate` = the class LOD gate; the loop runs when
        // it OR any resident override is active, and each instance re-checks per-prefab below.
        let badge_gate = class_visible("buildingBadge", z);
        let badge_want = self.toggle_buildings
            && (badge_gate || self.min_importance_zoom.is_some_and(|m| z >= m));

        if !tree_want && !prop_want && !badge_want {
            return;
        }

        let tree_code = class_code("tree");
        let veg_code = class_code("vegetation");
        let prop_code = class_code("prop");
        let rock_code = class_code("rockLarge");
        let building_code = class_code("building");

        let ids = self.draw_ids.clone();
        // Trees: pack every draw-set instance when under budget; clear when heatmap rung.
        let pack_trees = tree_want && !self.heatmap_trees;
        let mut prop_total = 0usize;
        let mut badge_total = 0usize;

        for id in &ids {
            let Some(chunk) = self.chunks.get(id) else {
                continue;
            };

            if pack_trees || prop_want {
                for &code in &[tree_code, veg_code, prop_code, rock_code] {
                    let class_ok = match code {
                        c if c == tree_code => class_visible("tree", z),
                        c if c == veg_code => class_visible("vegetation", z),
                        c if c == prop_code => class_visible("prop", z),
                        c if c == rock_code => class_visible("rockLarge", z),
                        _ => false,
                    };
                    if !class_ok {
                        continue;
                    }
                    let Some(rows) = chunk.rows_by_class.get(&code) else {
                        continue;
                    };
                    for &r in rows {
                        let r = r as usize;
                        let Some(info) = self.glyph_by_u16.get(&chunk.prefab_idx[r]).cloned()
                        else {
                            continue;
                        };
                        let is_tree_group = info.group == 0;
                        if is_tree_group {
                            if !pack_trees {
                                continue;
                            }
                        } else {
                            if !prop_want {
                                continue;
                            }
                            // Props keep a hard composition budget (no heatmap this slice).
                            if prop_total + badge_total >= INSTANCE_BUDGET {
                                continue;
                            }
                        }
                        let size =
                            size_with_min_px(f64::from(info.size_m), GLYPH_SIZE_MIN_PX, z) as f32;
                        let yaw = deck_angle_for_rotation_deg(f64::from(chunk.rotations[r]));
                        let px = chunk.positions[2 * r];
                        let py = chunk.positions[2 * r + 1];
                        if is_tree_group {
                            pack_icon_instance(
                                &mut self.tree_glyph_buf,
                                px,
                                py,
                                size,
                                yaw,
                                info.glyph_idx,
                                info.tint,
                            );
                        } else {
                            pack_icon_instance(
                                &mut self.prop_glyph_buf,
                                px,
                                py,
                                size,
                                yaw,
                                info.glyph_idx,
                                info.tint,
                            );
                            prop_total += 1;
                        }
                    }
                }
            }

            if badge_want {
                let Some(rows) = chunk.rows_by_class.get(&building_code) else {
                    continue;
                };
                for &r in rows {
                    if prop_total + badge_total >= INSTANCE_BUDGET {
                        break;
                    }
                    let r = r as usize;
                    let Some(binfo) = self.building_by_u16.get(&chunk.prefab_idx[r]) else {
                        continue;
                    };
                    // T-152.21 — per-prefab importanceZoom override: below the class badge band a
                    // landmark still emits when deck_zoom ≥ its importanceZoom; ordinary buildings
                    // (no override) fall back to the class gate. At/above the gate this is a no-op.
                    if !badge_gate && !binfo.importance_zoom.is_some_and(|iz| z >= iz) {
                        continue;
                    }
                    let cls = binfo.building_class.as_str();
                    if is_airfield_structure_class(cls) {
                        if !self.airfield_visible() {
                            continue;
                        }
                        let px = f64::from(chunk.positions[2 * r]);
                        let py = f64::from(chunk.positions[2 * r + 1]);
                        let Some(bbox) = self.airfield_bbox else {
                            continue;
                        };
                        if !point_in_bbox(px, py, bbox) {
                            continue;
                        }
                    }
                    let Some(key) = landmark_glyph_icon_key(cls) else {
                        continue;
                    };
                    let Some(&glyph_idx) = self.icon_key_to_idx.get(key) else {
                        continue;
                    };
                    let size = size_with_min_px(badge_size_meters(), BADGE_SIZE_MIN_PX, z) as f32;
                    pack_icon_instance(
                        &mut self.badge_glyph_buf,
                        chunk.positions[2 * r],
                        chunk.positions[2 * r + 1],
                        size,
                        0.0,
                        glyph_idx,
                        pack_rgba_u32([255, 255, 255, 255]),
                    );
                    badge_total += 1;
                }
            }
        }
    }

    /// Building fill instances (WORLD coords): 10 f32 each `[x, y, hx, hy, cos, sin, r, g, b, a]`.
    #[must_use]
    pub fn world_building_fill(&self) -> Vec<f32> {
        self.fill_buf.clone()
    }

    /// Building outline vertices (WORLD coords): 6 f32 each `[x, y, r, g, b, a]`, `LineList`.
    #[must_use]
    pub fn world_building_outline(&self) -> Vec<f32> {
        self.outline_buf.clone()
    }

    /// T-152.4 fence + pier strip triangle-list verts (WORLD coords): 6 f32/vert.
    #[must_use]
    pub fn world_fence_strips(&self) -> Vec<f32> {
        self.strip_buf.clone()
    }

    /// T-152.15 — exact fence strip instance count (one per fence prop OBB, gate G6).
    #[must_use]
    pub fn fence_strip_segment_count(&self) -> u32 {
        self.fence_strip_count
    }

    /// T-152.15 — exact pier/dock quay strip count (census gate G3; one per pier/dock instance).
    #[must_use]
    pub fn pier_strip_segment_count(&self) -> u32 {
        self.pier_strip_count
    }

    /// T-152.15 — bridge rail strip count (gate G5; 2 per bridge instance, Path A synthetic).
    #[must_use]
    pub fn bridge_rail_strip_count(&self) -> u32 {
        self.bridge_rail_count
    }

    /// Packed tree+vegetation icon instances (WORLD coords, 20 B each).
    #[must_use]
    pub fn world_tree_glyphs(&self) -> Vec<u8> {
        self.tree_glyph_buf.clone()
    }

    /// Packed prop+rockLarge icon instances (WORLD coords, 20 B each).
    #[must_use]
    pub fn world_prop_glyphs(&self) -> Vec<u8> {
        self.prop_glyph_buf.clone()
    }

    /// Packed building-badge icon instances (WORLD coords, 20 B each).
    #[must_use]
    pub fn world_badge_glyphs(&self) -> Vec<u8> {
        self.badge_glyph_buf.clone()
    }

    #[must_use]
    pub fn tree_glyph_count(&self) -> u32 {
        (self.tree_glyph_buf.len() / super::glyph_math::ICON_INSTANCE_STRIDE) as u32
    }

    #[must_use]
    pub fn prop_glyph_count(&self) -> u32 {
        (self.prop_glyph_buf.len() / super::glyph_math::ICON_INSTANCE_STRIDE) as u32
    }

    #[must_use]
    pub fn badge_glyph_count(&self) -> u32 {
        (self.badge_glyph_buf.len() / super::glyph_math::ICON_INSTANCE_STRIDE) as u32
    }

    /// Test/diagnostic: glyph lookup entries for a compose group (0 tree, 1 prop, 2 building).
    #[cfg(test)]
    #[must_use]
    pub fn glyph_lookup_len_for_group(&self, group: u8) -> usize {
        self.glyph_by_u16
            .values()
            .filter(|info| info.group == group)
            .count()
    }

    /// Test/diagnostic: atlas index for an icon key (when registered).
    #[cfg(test)]
    #[must_use]
    pub fn glyph_idx_for_key(&self, key: &str) -> Option<u16> {
        self.icon_key_to_idx.get(key).copied()
    }

    /// Sorted draw-set chunk ids (strict visible ∩ pinned ∩ cells).
    #[must_use]
    pub fn draw_ids(&self) -> &[String] {
        &self.draw_ids
    }

    #[must_use]
    pub fn chunks_draw(&self) -> u32 {
        self.draw_ids.len() as u32
    }

    /// Exact-count tree heatmap rung active.
    #[must_use]
    pub fn heatmap_trees_active(&self) -> bool {
        self.heatmap_trees
    }

    /// T-152.14 — effective forest-mass (fill/outline) visibility: the residency handoff that keeps
    /// the green mass alive until tree glyphs actually pack (audit A3). Below the glyph band
    /// (`class_visible("forestFill")`, z < 0) mass shows as always. At z ≥ 0 mass is normally off
    /// (glyphs replace it), but it PERSISTS while the wanted tree lane is not drawing glyphs — i.e.
    /// the heatmap rung is active or the glyph buffer packed empty — so zooming into dense forest
    /// never leaves a blank band. Read after `rebuild_glyph_buffers` so `tree_glyph_buf` is final.
    /// `class_visible("forestFill")` stays the pure-zoom oracle; this is the state-aware decision.
    #[must_use]
    pub fn forest_fill_effective(&self) -> bool {
        if class_visible("forestFill", self.deck_zoom) {
            return true;
        }
        self.toggle_trees && (self.heatmap_trees || self.tree_glyph_buf.is_empty())
    }

    #[must_use]
    pub fn exact_tree_count_draw(&self) -> u32 {
        self.exact_tree_count
    }

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

    /// Resident chunk ids (sorted) — parity/debug surface.
    #[must_use]
    pub fn resident_chunk_ids(&self) -> Vec<String> {
        let mut v: Vec<String> = self.chunks.keys().cloned().collect();
        v.sort();
        v
    }

    /// Ordered eviction victims since construction — parity surface (Class S eviction-order log).
    #[must_use]
    pub fn eviction_log(&self) -> Vec<String> {
        self.eviction_log.clone()
    }

    /// Total building instances across the pinned chunks (== JS `getWorldBuildings().length`).
    #[must_use]
    pub fn pinned_building_count(&self) -> u32 {
        self.pinned_ids
            .iter()
            .map(|id| self.building_counts.get(id).copied().unwrap_or(0))
            .sum()
    }

    #[must_use]
    pub fn chunks_resident(&self) -> usize {
        self.chunks.len()
    }

    #[must_use]
    pub fn frames_over_budget(&self) -> u64 {
        self.frames_over_budget
    }

    /// Chunks requested but not yet delivered (T-151.4.1 — empty-push / abort diagnostics).
    #[must_use]
    pub fn inflight_count(&self) -> usize {
        self.inflight.len()
    }

    /// Drop all in-flight marks (T-151.4.1). Call when a fetch is aborted so the next
    /// `set_viewport` can re-request those ids; without this, aborted ids stay excluded forever.
    pub fn clear_inflight(&mut self) {
        self.inflight.clear();
    }

    /// Release one in-flight mark after a soft fetch failure (host may retry next settle).
    pub fn release_inflight(&mut self, id: &str) {
        self.inflight.remove(id);
    }

    /// Instance count of a resident chunk (`None` if not resident).
    #[must_use]
    pub fn resident_instance_count(&self, id: &str) -> Option<u32> {
        self.chunks.get(id).map(|c| c.count)
    }

    /// Drop a resident (or empty-stub) chunk so the next `set_viewport` re-requests it.
    /// Used by the Leptos host to recover from a soft HTTP failure that must not be cached
    /// as a permanent empty stub (tree-glyph zoom probes need real instance rows).
    pub fn invalidate_chunk(&mut self, id: &str) {
        if self.chunks.remove(id).is_some() {
            self.index.remove_chunk(id);
            self.building_counts.remove(id);
            self.last_used.remove(id);
            self.inserted_seq.remove(id);
            self.content_epoch += 1;
        }
        self.inflight.remove(id);
        // Manual recovery path — the mark and the failure count both reset (T-173 P3).
        self.known_empty.remove(id);
        self.fetch_failures.remove(id);
    }

    /// Mark ids as in-flight (not yet resident). Used after `clear_inflight` when starting a
    /// replacement fetch so concurrent same-key `set_viewport` does not re-queue them.
    pub fn mark_inflight(&mut self, ids: &[String]) {
        for id in ids {
            if !self.chunks.contains_key(id) {
                self.inflight.insert(id.clone());
            }
        }
    }

    /// True when every pinned id is either resident or known-empty (present in `chunks`).
    /// Empty pin set (gate closed) counts as settled.
    #[must_use]
    pub fn pin_settled(&self) -> bool {
        self.pinned_ids
            .iter()
            .all(|id| self.chunks.contains_key(id))
    }

    /// Additive residency stats JSON (separate from `RenderEngine::stats()`).
    /// T-151.4.1: appends `inflight_count` + `pin_settled` (prior keys unchanged).
    /// T-151.8: appends `chunks_draw`, `exact_tree_count`, `heatmap_trees` (prior keys unchanged).
    #[must_use]
    pub fn stats_json(&self) -> String {
        format!(
            "{{\"chunks_resident\":{},\"chunks_pinned\":{},\"chunks_applied\":{},\"apply_frames\":{},\"apply_budget_ms_last\":{},\"max_apply_ms\":{},\"frames_over_budget\":{},\"building_instances\":{},\"index_size\":{},\"inflight_count\":{},\"pin_settled\":{},\"chunks_draw\":{},\"exact_tree_count\":{},\"heatmap_trees\":{},\"buffers_revision\":{},\"glyph_recomposes\":{},\"fill_recomposes\":{},\"known_empty_count\":{}}}",
            self.chunks.len(),
            self.pinned_ids.len(),
            self.chunks_applied,
            self.apply_frames,
            self.apply_budget_ms_last,
            self.max_apply_ms,
            self.frames_over_budget,
            self.pinned_building_count(),
            self.index.size(),
            self.inflight.len(),
            self.pin_settled(),
            self.draw_ids.len(),
            self.exact_tree_count,
            self.heatmap_trees,
            self.buffers_revision,
            self.glyph_recomposes,
            self.fill_recomposes,
            self.known_empty.len(),
        )
    }

    /// T-173 — monotonic revision of the GPU-facing output buffers (see field doc). Hosts compare
    /// against their last-pushed value to skip redundant clone+upload passes.
    #[must_use]
    pub fn buffers_revision(&self) -> u64 {
        self.buffers_revision
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::density_ladder::density_texel_sum_for_draw_ids;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    fn gzip(text: &str) -> Vec<u8> {
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(text.as_bytes()).unwrap();
        enc.finish().unwrap()
    }

    /// A residency over a full 25×25 Everon grid (one building prefab id 9), every cell present.
    fn setup() -> WorldResidency {
        let mut r = WorldResidency::new();
        r.load_manifest_json(
            r#"{ "worldBounds": [0,0,12800,12800], "objects": { "prefabsPath": "p", "chunksPath": "c", "chunkSizeM": 512 } }"#,
        )
        .unwrap();
        r.load_prefabs_gz(
            br#"{ "prefabs": [ { "prefabId": 9, "kind": "building", "class": "residential", "spatial": { "halfExtentsM": { "x": 5, "y": 5, "z": 4 } } } ] }"#,
        )
        .unwrap();
        // Full grid cells (0..24)².
        let mut cells = String::from("{\"cells\":[");
        for cy in 0..25 {
            for cx in 0..25 {
                if cx != 0 || cy != 0 {
                    cells.push(',');
                }
                cells.push_str(&format!(
                    "{{\"cx\":{cx},\"cy\":{cy},\"path\":\"objects/chunks/{cx}_{cy}.json.gz\"}}"
                ));
            }
        }
        cells.push_str("]}");
        r.load_chunk_index_json(&cells).unwrap();
        r
    }

    /// One building per chunk at its cell origin + (100.5, 200.25).
    fn chunk_bytes(id: &str) -> Vec<u8> {
        let mut parts = id.split('_');
        let cx: f64 = parts.next().unwrap().parse().unwrap();
        let cy: f64 = parts.next().unwrap().parse().unwrap();
        let x = cx * 512.0 + 100.5;
        let y = cy * 512.0 + 200.25;
        gzip(&format!("{{\"instances\":[[9,{x},{y},10,45]]}}"))
    }

    /// set_viewport + deliver every missing chunk + end the apply frame (fast clock).
    fn drive(r: &mut WorldResidency, bbox: [f64; 4]) {
        let missing = r.set_viewport(bbox[0], bbox[1], bbox[2], bbox[3], -2.0);
        for id in &missing {
            r.ingest_chunk_gz(id, &chunk_bytes(id)).unwrap();
        }
        if !missing.is_empty() {
            r.end_apply_frame(0.0);
        }
    }

    // T-152.14 — synthetic dense-forest fixture (self-contained; no map assets).

    /// Overwrite a resident chunk with `n` tree rows (prefab 0 = the `tree-conifer` glyph below).
    /// Positions are irrelevant to the area-fraction count (it uses the chunk id → world bbox).
    fn inject_trees(r: &mut WorldResidency, id: &str, n: usize) {
        let c = r.chunks.get_mut(id).unwrap();
        c.count = n as u32;
        c.positions = vec![0.0; n * 2];
        c.prefab_idx = vec![0; n];
        c.rotations = vec![0.0; n];
        c.z = vec![0.0; n];
        c.cls_codes = vec![class_code("tree"); n];
        c.rows_by_class.clear();
        c.rows_by_class
            .insert(class_code("tree"), (0..n as u32).collect());
        // Real chunk-content changes only ever happen via `insert_chunk`, which bumps the epoch;
        // this direct-mutation test helper must do the same so the T-173 P2 compose memo sees the
        // change and recomposes (otherwise an identical-viewport `refresh` correctly memo-hits).
        r.content_epoch += 1;
    }

    /// Everon-sized residency whose one prefab (id 0) is a tree with a real glyph atlas key, so
    /// injected tree rows actually pack into `tree_glyph_buf` (`tree_glyph_count() > 0`).
    fn dense_forest_setup() -> WorldResidency {
        let mut r = WorldResidency::new();
        r.load_manifest_json(
            r#"{ "worldBounds": [0,0,12800,12800], "objects": { "prefabsPath": "p", "chunksPath": "c", "chunkSizeM": 512 } }"#,
        )
        .unwrap();
        r.load_prefabs_gz(
            br##"{ "prefabs": [ { "prefabId": 0, "kind": "tree", "class": "conifer", "spatial": { "halfExtentsM": { "x": 1.2, "y": 1.2, "z": 6 }, "heightM": 12 }, "render": { "iconKey": "tree-conifer", "baseSizePx": 18, "defaultColor": "#2d5a27" } } ] }"##,
        )
        .unwrap();
        let mut cells = String::from("{\"cells\":[");
        for cy in 0..25 {
            for cx in 0..25 {
                if cx != 0 || cy != 0 {
                    cells.push(',');
                }
                cells.push_str(&format!(
                    "{{\"cx\":{cx},\"cy\":{cy},\"path\":\"objects/chunks/{cx}_{cy}.json.gz\"}}"
                ));
            }
        }
        cells.push_str("]}");
        r.load_chunk_index_json(&cells).unwrap();
        r.set_glyph_key_map(&["tree-conifer".to_string()]);
        r
    }

    #[test]
    fn requests_exactly_the_chunk_math_set() {
        let mut r = setup();
        let missing = r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
        let expected =
            chunk_ids_for_viewport([2000.0, 2000.0, 2200.0, 2200.0], r.terrain, 512.0, 0);
        assert_eq!(missing, expected);
    }

    // ── T-173 P2/P3 — compose memo + ingest disposition ─────────────────────────

    /// The glyph memo skips recompose (revision stable) when nothing changed, and bumps on a real
    /// content/zoom change.
    #[test]
    fn compose_memo_stable_then_bumps() {
        let mut r = dense_forest_setup();
        drive(&mut r, [2000.0, 2000.0, 2200.0, 2200.0]);
        let rev0 = r.buffers_revision();
        // Identical set_viewport → memo hit → no recompose, no revision bump.
        r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
        r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
        assert_eq!(
            r.buffers_revision(),
            rev0,
            "identical viewport must not recompose"
        );
        // A zoom change (glyph min-px sizing depends on it) must invalidate the memo.
        r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -1.0);
        assert!(r.buffers_revision() > rev0, "zoom change must recompose");
    }

    /// A newly-ingested chunk in the draw set bumps the revision even though the viewport rect is
    /// unchanged (content epoch is part of the memo key).
    #[test]
    fn compose_memo_invalidates_on_new_chunk() {
        let mut r = setup();
        // Pin a 2×2 area but deliver only some chunks, then deliver the rest under the same pin.
        let missing = r.set_viewport(0.0, 0.0, 900.0, 900.0, -2.0);
        assert!(missing.len() >= 2);
        r.ingest_chunk_gz(&missing[0], &chunk_bytes(&missing[0]))
            .unwrap();
        r.end_apply_frame(0.0);
        r.set_viewport(0.0, 0.0, 900.0, 900.0, -2.0);
        let rev1 = r.buffers_revision();
        r.ingest_chunk_gz(&missing[1], &chunk_bytes(&missing[1]))
            .unwrap();
        r.end_apply_frame(0.0);
        r.set_viewport(0.0, 0.0, 900.0, 900.0, -2.0);
        assert!(
            r.buffers_revision() > rev1,
            "a freshly-ingested chunk under the same pin must recompose"
        );
    }

    /// A well-formed chunk with zero instances is marked known-empty: kept resident, never evicted,
    /// never re-requested by a later same-key `set_viewport`.
    #[test]
    fn parsed_empty_chunk_is_known_empty_and_not_refetched() {
        let mut r = setup();
        let missing = r.set_viewport(0.0, 0.0, 200.0, 200.0, -2.0);
        let id = missing[0].clone();
        let out = r
            .ingest_chunk_gz(&id, &gzip(r#"{"instances":[]}"#))
            .unwrap();
        assert_eq!(out, IngestOutcome::ParsedEmpty);
        assert!(r.stats_json().contains("\"known_empty_count\":1"));
        // Same pin → the empty chunk is resident, so it is NOT in the missing set again.
        let again = r.set_viewport(0.0, 0.0, 200.0, 200.0, -2.0);
        assert!(
            !again.contains(&id),
            "known-empty chunk must not be re-requested"
        );
    }

    /// A payload that is not chunk-shaped is a `ShapeMismatch`, retried up to the cap, then cached
    /// as an undelivered stub (stops thrashing).
    #[test]
    fn shape_mismatch_retries_to_cap_then_caches() {
        let mut r = setup();
        let missing = r.set_viewport(0.0, 0.0, 200.0, 200.0, -2.0);
        let id = missing[0].clone();
        for _ in 0..(FETCH_FAILURE_CAP - 1) {
            let out = r.ingest_chunk_gz(&id, &gzip("{}")).unwrap();
            assert_eq!(out, IngestOutcome::ShapeMismatch);
            // Below the cap → not resident (will retry).
            assert!(r.resident_instance_count(&id).is_none());
        }
        // The capping failure caches an (empty) stub so the id stops being re-requested.
        r.ingest_chunk_gz(&id, &gzip("{}")).unwrap();
        assert!(r.resident_instance_count(&id).is_some());
        let again = r.set_viewport(0.0, 0.0, 200.0, 200.0, -2.0);
        assert!(!again.contains(&id));
    }

    /// A new pin epoch resets the failure counters (a transient outage during one hold must not
    /// permanently cap chunks the operator pans back to).
    #[test]
    fn fetch_failures_reset_on_new_pin_key() {
        let mut r = setup();
        let m0 = r.set_viewport(0.0, 0.0, 200.0, 200.0, -2.0);
        let id = m0[0].clone();
        r.ingest_chunk_gz(&id, &gzip("{}")).unwrap(); // 1 failure
        // Pan far away (new pin key), then back — the counter reset means the id is requestable.
        r.set_viewport(6000.0, 6000.0, 6200.0, 6200.0, -2.0);
        let back = r.set_viewport(0.0, 0.0, 200.0, 200.0, -2.0);
        assert!(
            back.contains(&id),
            "failure counter must reset across pin epochs"
        );
    }

    /// T-151.4.1: aborted fetch clears inflight; same-key set_viewport re-requests undelivered.
    #[test]
    fn clear_inflight_allows_same_key_rerequest() {
        let mut r = setup();
        let missing = r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
        assert!(!missing.is_empty());
        assert!(r.inflight_count() > 0);
        assert!(!r.pin_settled());
        // Simulate abort: wipe inflight without delivering.
        r.clear_inflight();
        assert_eq!(r.inflight_count(), 0);
        // Same pin key, unsettled + empty inflight → re-request.
        let again = r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
        assert_eq!(again, missing);
        assert_eq!(r.inflight_count(), missing.len());
        // Deliver → settled.
        for id in &again {
            r.ingest_chunk_gz(id, &chunk_bytes(id)).unwrap();
        }
        r.end_apply_frame(0.0);
        assert!(r.pin_settled());
        assert!(r.pinned_building_count() > 0);
        // Settled same key → empty missing.
        assert!(
            r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0)
                .is_empty()
        );
    }

    #[test]
    fn skip_below_building_band_and_unchanged_set() {
        let mut r = setup();
        assert!(
            r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -3.0)
                .is_empty()
        ); // below band
        let first = r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
        assert!(!first.is_empty());
        // Same chunk rect → early-exit, no new request.
        assert!(
            r.set_viewport(2001.0, 2001.0, 2201.0, 2201.0, -2.0)
                .is_empty()
        );
    }

    #[test]
    fn building_count_and_buffers_track_pins() {
        let mut r = setup();
        drive(&mut r, [2000.0, 2000.0, 2200.0, 2200.0]);
        let pinned = r.pinned_ids.len() as u32;
        assert!(pinned > 0);
        // One building per chunk → pinned_building_count == pinned chunk count.
        assert_eq!(r.pinned_building_count(), pinned);
        // Fill buffer = 10 f32 per building; outline = 48 f32 per building (8 verts × 6).
        assert_eq!(r.world_building_fill().len(), 10 * pinned as usize);
        assert_eq!(r.world_building_outline().len(), 48 * pinned as usize);
    }

    #[test]
    fn lru_caps_and_never_evicts_pinned() {
        let mut r = setup();
        // First viewport (pinned this stop).
        drive(&mut r, [200.0, 200.0, 400.0, 400.0]);
        let first_ids: Vec<String> = r.pinned_ids.clone();
        // Sweep far across the island: each stop pins a small set; cap = max(64, 3×pinned).
        for i in 0..10 {
            let x = 3000.0 + f64::from(i) * 1024.0;
            drive(&mut r, [x, 6000.0, x + 200.0, 6200.0]);
        }
        // Resident never exceeds the cap.
        let cap = LRU_MIN_CHUNKS.max(3 * r.pinned_ids.len());
        assert!(r.chunks_resident() <= cap);
        // The current viewport's chunks are all resident (pinned never evicted).
        for id in &r.pinned_ids {
            assert!(r.chunks.contains_key(id), "pinned {id} must stay resident");
        }
        // With cap 64 and a 10-stop sweep well beyond it, the first stop aged out.
        let first_evicted = first_ids.iter().any(|id| !r.chunks.contains_key(id));
        assert!(first_evicted, "first viewport should have been evicted");
    }

    #[test]
    fn eviction_order_is_ascending_last_used() {
        let mut r = setup();
        // Drive a sweep that forces evictions, then check the log is oldest-first. Because
        // last_used is assigned by a strictly-increasing tick, "oldest" == smallest tick, and the
        // eviction loop pops candidates ascending; the log must be non-decreasing in the tick each
        // id last held. We reconstruct that by re-deriving: a victim evicted earlier had a smaller
        // last_used than one evicted later within the same evict() call.
        for i in 0..12 {
            let x = 500.0 + f64::from(i) * 1024.0;
            drive(&mut r, [x, 500.0, x + 200.0, 700.0]);
        }
        let log = r.eviction_log();
        assert!(!log.is_empty(), "sweep should evict");
        // Every evicted id is no longer resident.
        for id in &log {
            assert!(!r.chunks.contains_key(id));
        }
    }

    #[test]
    fn budget_accounting_matches_elapsed_sequence() {
        let mut r = setup();
        for ms in [1.0_f64, 5.0, 3.0, 6.0, 4.0] {
            r.end_apply_frame(ms);
        }
        assert_eq!(r.apply_frames, 5);
        assert_eq!(r.frames_over_budget(), 2); // 5.0 and 6.0 exceed 4.0 (4.0 is not > 4.0)
        assert!((r.max_apply_ms - 6.0).abs() < f64::EPSILON);
        assert!((r.apply_budget_ms_last - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn pick_finds_resident_building() {
        let mut r = setup();
        drive(&mut r, [2000.0, 2000.0, 2200.0, 2200.0]);
        // Chunk 4_4 origin (2048,2048) + (100.5,200.25) = (2148.5, 2248.25); building code 0.
        let hit = r.pick_nearest(2148.5, 2248.25, 5.0, Some(1 << 0));
        assert!(hit.is_some(), "should pick the building at chunk 4_4");
    }

    /// Class S: draw_chunk_ids == strict rect ∩ pinned ∩ cells (sorted equality).
    #[test]
    fn class_s_draw_set_equals_strict_reference() {
        let mut r = setup();
        // Pin a wide preload region via set_viewport.
        drive(&mut r, [1500.0, 1500.0, 3500.0, 3500.0]);
        assert!(r.pinned_ids.len() >= 9);

        let strict: Bbox = [2048.0, 2048.0, 3072.0, 3072.0];
        let draw = r.draw_chunk_ids(strict);
        let mut reference = chunk_ids_for_rect(chunk_rect_for_bbox(strict, r.terrain, 512.0));
        if let Some(cells) = &r.cell_ids {
            reference.retain(|id| cells.contains(id));
        }
        reference.retain(|id| r.pinned_set.contains(id));
        reference.sort();
        assert_eq!(draw, reference);

        for id in &draw {
            assert!(r.pinned_set.contains(id));
        }

        // Preload pin ⊃ draw-set: viewport pin with preload expands beyond strict rect.
        let pin_via_viewport = chunk_ids_for_viewport(strict, r.terrain, 512.0, 0);
        let mut pin_set: HashSet<String> = pin_via_viewport.into_iter().collect();
        if let Some(cells) = &r.cell_ids {
            pin_set.retain(|id| cells.contains(id));
        }
        // draw is strict subset of preload viewport set whenever preload adds a ring.
        assert!(
            pin_set.len() > draw.len(),
            "preload must expand beyond strict draw (pin {} vs draw {})",
            pin_set.len(),
            draw.len()
        );
        for id in &draw {
            assert!(pin_set.contains(id), "draw id {id} must be in preload pin");
        }
    }

    /// Class R (T-152.14 refined): swap counts viewport-VISIBLE trees. Chunk `4_4` sits fully
    /// inside `[2048,2048,3072,3072]` (area-frac 1), so over-visible-budget still swaps to the
    /// heatmap rung (the ladder stays reachable for true mega-viewports) and clears tree glyphs;
    /// under budget the density texel sum still equals the whole-chunk exact.
    #[test]
    fn class_r_heatmap_swap_and_full_pack() {
        let mut r = setup();
        // Inject synthetic tree rows into two resident chunks after a pin.
        drive(&mut r, [2048.0, 2048.0, 3072.0, 3072.0]);
        r.last_viewport = [2048.0, 2048.0, 3072.0, 3072.0];
        r.deck_zoom = 0.0; // tree glyphs visible
        r.toggle_trees = true;

        let draw = r.draw_chunk_ids(r.last_viewport);
        assert!(!draw.is_empty());
        // Put 10 trees in first draw chunk (4_4, fully in view) — under budget.
        let id0 = draw[0].clone();
        inject_trees(&mut r, &id0, 10);
        r.refresh_draw_set_and_glyphs();
        let exact = exact_tree_count(&r.chunks, &r.draw_ids);
        assert_eq!(exact, 10);
        assert!(!r.heatmap_trees_active());
        // Texel sum Class R (grid stays whole-chunk exact).
        let sum = density_texel_sum_for_draw_ids(&r.density_grid, r.density_grid_w, &r.draw_ids);
        assert_eq!(sum, exact as u64);

        // Force over-budget: inflate the fully-visible chunk to INSTANCE_BUDGET + 1 (area-frac 1).
        inject_trees(&mut r, &id0, INSTANCE_BUDGET + 1);
        r.refresh_draw_set_and_glyphs();
        assert!(r.heatmap_trees_active());
        assert_eq!(r.tree_glyph_count(), 0);
        assert_eq!(r.exact_tree_count_draw() as usize, INSTANCE_BUDGET + 1);
        let sum2 = density_texel_sum_for_draw_ids(&r.density_grid, r.density_grid_w, &r.draw_ids);
        assert_eq!(sum2, (INSTANCE_BUDGET + 1) as u64);
        // A3 handoff: glyphs are heatmap-cleared at z ≥ 0, so forest mass persists — not blank.
        assert!(r.forest_fill_effective());
    }

    /// T-152.14 (A2): a chunk barely on-screen no longer drags the swap over budget. Chunk `4_4`
    /// holds `INSTANCE_BUDGET + 1` trees but only ~10 % is visible → refined count ≈ 15 k → glyphs,
    /// not heatmap. (This is the whole-chunk-count blank-band bug, now fixed.)
    #[test]
    fn class_r_partial_coverage_no_swap() {
        let mut r = setup();
        drive(&mut r, [2048.0, 2048.0, 2560.0, 2560.0]); // pin chunk 4_4 (+ neighbours)
        inject_trees(&mut r, "4_4", INSTANCE_BUDGET + 1);
        r.deck_zoom = 0.0;
        r.toggle_trees = true;
        // Sliver viewport: x 2048..2099.2 of chunk 4_4's [2048,2560] = 51.2/512 ≈ 0.1 area-frac.
        r.last_viewport = [2048.0, 2048.0, 2099.2, 2560.0];
        r.refresh_draw_set_and_glyphs();
        let visible = visible_tree_count(&r.chunks, &r.draw_ids, r.last_viewport, 512.0);
        assert!(visible <= INSTANCE_BUDGET, "sliver visible = {visible}");
        assert!(!r.heatmap_trees_active());
        // Whole-chunk exact is still the full census (grid parity unchanged).
        assert_eq!(r.exact_tree_count_draw() as usize, INSTANCE_BUDGET + 1);
    }

    /// T-152.14 (G2): swap hysteresis over `[0.85×budget, budget]` kills boundary flicker. Chunk
    /// `4_4` is fully in view (area-frac 1) so `visible == injected count`; drive budget+1 (enter)
    /// → budget−1 (stay) → 0.85×budget−1 (exit).
    #[test]
    fn class_r_heatmap_hysteresis() {
        let mut r = setup();
        drive(&mut r, [2048.0, 2048.0, 2560.0, 2560.0]);
        r.deck_zoom = 0.0;
        r.toggle_trees = true;
        r.last_viewport = [2048.0, 2048.0, 2560.0, 2560.0]; // fully contains chunk 4_4
        let reenter = INSTANCE_BUDGET * 85 / 100;

        inject_trees(&mut r, "4_4", INSTANCE_BUDGET + 1);
        r.refresh_draw_set_and_glyphs();
        assert!(r.heatmap_trees_active(), "enter above budget");

        inject_trees(&mut r, "4_4", INSTANCE_BUDGET - 1); // in the band → stays
        r.refresh_draw_set_and_glyphs();
        assert!(
            r.heatmap_trees_active(),
            "stay in heatmap inside the hysteresis band"
        );

        inject_trees(&mut r, "4_4", reenter - 1); // below 0.85× → exit
        r.refresh_draw_set_and_glyphs();
        assert!(!r.heatmap_trees_active(), "exit below 0.85×budget");
    }

    /// T-152.14 (G3/G4): never-blank property gate over the full detail-zoom ladder. On a dense
    /// forest fixture, for every z ∈ {0, 0.5, …, 6} at least one of {tree glyphs, forest mass,
    /// heatmap+grid} is present — under budget glyphs pack and mass is off (handoff), over budget
    /// the heatmap owns the rung and mass persists.
    #[test]
    fn property_never_blank_zoom_ladder() {
        let vp = [2048.0, 2048.0, 3072.0, 3072.0]; // 9 draw chunks; 4 fully covered (4-5 × 4-5)
        // 50/chunk → 200 visible (glyphs); 40 000/chunk → 160 000 visible (heatmap + handoff).
        for &per_chunk in &[50usize, 40_000usize] {
            let mut r = dense_forest_setup();
            drive(&mut r, vp);
            for id in r.draw_chunk_ids(vp) {
                inject_trees(&mut r, &id, per_chunk);
            }
            r.toggle_trees = true;
            for step in 0..=12u32 {
                let z = f64::from(step) * 0.5; // 0.0 … 6.0
                r.deck_zoom = z;
                r.last_viewport = vp;
                r.refresh_draw_set_and_glyphs();

                let glyphs = r.tree_glyph_count() > 0;
                let fill = r.forest_fill_effective();
                let heat = r.heatmap_trees_active();
                let grid_nonzero = r.density_grid.iter().any(|&v| v > 0);
                assert!(
                    glyphs || fill || (heat && grid_nonzero),
                    "blank band @ z={z}, per_chunk={per_chunk}"
                );

                let visible = visible_tree_count(&r.chunks, &r.draw_ids, vp, 512.0);
                if visible <= INSTANCE_BUDGET {
                    assert!(glyphs, "glyphs empty under budget @ z={z}");
                    assert!(!fill, "mass must be off when glyphs pack @ z={z}"); // G4
                } else {
                    assert!(heat, "heatmap must own the rung over budget @ z={z}");
                    assert!(fill, "mass must persist under heatmap @ z={z}"); // G4 / A3
                }
            }
        }
    }

    #[test]
    fn class_r_chunks_draw_matches_draw_ids_len() {
        let mut r = setup();
        drive(&mut r, [2048.0, 2048.0, 3072.0, 3072.0]);
        assert_eq!(r.chunks_draw() as usize, r.draw_ids().len());
        let stats: serde_json::Value = serde_json::from_str(&r.stats_json()).unwrap();
        assert_eq!(
            stats["chunks_draw"].as_u64().unwrap(),
            r.draw_ids().len() as u64
        );
    }
}

/// T-151.11.3 tests — ingest-frame budget policy (B-04) + buildings-toggle lane hide (P-04).
#[cfg(test)]
mod t151_11_3_tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    fn gzip(text: &str) -> Vec<u8> {
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(text.as_bytes()).unwrap();
        enc.finish().unwrap()
    }

    fn residency_with_one_building() -> WorldResidency {
        let mut r = WorldResidency::new();
        r.load_manifest_json(
            r#"{ "worldBounds": [0,0,12800,12800], "objects": { "prefabsPath": "p", "chunksPath": "c", "chunkSizeM": 512 } }"#,
        )
        .unwrap();
        r.load_prefabs_gz(
            br#"{ "prefabs": [ { "prefabId": 9, "kind": "building", "class": "residential", "spatial": { "halfExtentsM": { "x": 5, "y": 5, "z": 4 } } } ] }"#,
        )
        .unwrap();
        let missing = r.set_viewport(0.0, 0.0, 600.0, 600.0, -2.0);
        assert!(!missing.is_empty());
        for id in &missing {
            r.ingest_chunk_gz(id, &gzip(r#"{"instances":[[9,100.5,200.25,10,45]]}"#))
                .unwrap();
        }
        r.end_apply_frame(0.0);
        r
    }

    #[test]
    fn ingest_budget_policy_is_core_owned() {
        let mut r = WorldResidency::new();
        // No open frame → never exhausted (callers may always ingest ≥ 1 chunk).
        assert!(!r.ingest_budget_exhausted_at(1_000.0));
        r.begin_ingest_frame_at(1_000.0);
        assert!(!r.ingest_budget_exhausted_at(1_000.0 + APPLY_BUDGET_MS - 0.1));
        assert!(r.ingest_budget_exhausted_at(1_000.0 + APPLY_BUDGET_MS));
        // Closing records the elapsed into the frame stats (over-budget counter increments).
        let before = r.frames_over_budget();
        r.end_ingest_frame_at(1_000.0 + APPLY_BUDGET_MS + 2.0);
        assert_eq!(r.frames_over_budget(), before + 1);
        // Frame closed → not exhausted again until reopened.
        assert!(!r.ingest_budget_exhausted_at(10_000.0));
    }

    #[test]
    fn buildings_toggle_hides_and_restores_whole_lane() {
        let mut r = residency_with_one_building();
        assert!(!r.world_building_fill().is_empty());
        assert!(!r.world_building_outline().is_empty());
        assert!(r.buildings_visible());

        r.set_glyph_toggles(true, false, false); // buildings OFF
        assert!(
            r.world_building_fill().is_empty(),
            "fill must empty on toggle-off (P-04)"
        );
        assert!(
            r.world_building_outline().is_empty(),
            "outline must empty on toggle-off"
        );
        assert!(!r.buildings_visible());

        r.set_glyph_toggles(true, false, true); // buildings back ON
        assert!(
            !r.world_building_fill().is_empty(),
            "fill must repopulate on toggle-on"
        );
        assert!(r.buildings_visible());
    }

    #[test]
    fn buildings_visible_respects_zoom_gate() {
        let mut r = residency_with_one_building();
        assert!(r.buildings_visible()); // zoom −2 ≥ −2.5 gate
        let _ = r.set_viewport(0.0, 0.0, 600.0, 600.0, -3.0); // below BUILDING_MIN_ZOOM
        assert!(!r.buildings_visible());
    }
}

/// T-152.3 — landmark building glyph wiring (Class R vs TS oracle policy).
#[cfg(test)]
mod t152_3_tests {
    use super::super::glyph_math::{
        BUILDING_CLASSES, badge_icon_key, building_icon_key, landmark_glyph_icon_key,
    };
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::path::PathBuf;

    const FIXTURE_CHUNK: &str = "2_12";
    const N_MIN_BUILDING_GLYPH_LOOKUP: usize = 15;

    fn map_assets() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/map-assets")
    }

    fn glyph_keys_from_manifest() -> Vec<String> {
        let raw =
            fs::read_to_string(map_assets().join("glyphs/manifest.json")).expect("glyphs manifest");
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let mut keys: Vec<String> = v["glyphs"]
            .as_object()
            .expect("glyphs object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    }

    fn load_everon_residency() -> WorldResidency {
        let everon = map_assets().join("everon");
        let mut r = WorldResidency::new();
        r.load_manifest_json(
            &fs::read_to_string(everon.join("manifest.json")).expect("everon manifest"),
        )
        .unwrap();
        r.load_prefabs_gz(
            &fs::read(everon.join("objects/prefabs.json.gz")).expect("everon prefabs"),
        )
        .unwrap();
        r.load_chunk_index_json(
            &fs::read_to_string(everon.join("objects/chunks/manifest.json")).expect("chunk index"),
        )
        .unwrap();
        r.set_glyph_key_map(&glyph_keys_from_manifest());
        r
    }

    fn building_class_by_prefab_u16() -> HashMap<u16, String> {
        let everon = map_assets().join("everon");
        let raw = super::super::store::bytes_to_json(
            &fs::read(everon.join("objects/prefabs.json.gz")).unwrap(),
        )
        .unwrap();
        let mut out = HashMap::new();
        for row in narrow_prefab_rows(&raw) {
            if row.kind != "building" {
                continue;
            }
            let pid = row.prefab_id;
            if (0.0..65536.0).contains(&pid) && pid.fract() == 0.0 {
                out.insert(pid as u16, row.class);
            }
        }
        out
    }

    /// T-152.21 — u16 prefab id → `render.importanceZoom` (only building prefabs that carry it).
    /// Independent oracle for the badge override: read straight from the compiled fixture, mirroring
    /// how `narrow_prefab_rows` feeds `BuildingPrefabInfo.importance_zoom` in the live path.
    fn building_importance_by_prefab_u16() -> HashMap<u16, f64> {
        let everon = map_assets().join("everon");
        let raw = super::super::store::bytes_to_json(
            &fs::read(everon.join("objects/prefabs.json.gz")).unwrap(),
        )
        .unwrap();
        let mut out = HashMap::new();
        for row in narrow_prefab_rows(&raw) {
            if row.kind != "building" {
                continue;
            }
            let pid = row.prefab_id;
            if !(0.0..65536.0).contains(&pid) || pid.fract() != 0.0 {
                continue;
            }
            if let Some(iz) = row.importance_zoom {
                out.insert(pid as u16, iz);
            }
        }
        out
    }

    fn drive_fixture_chunk(r: &mut WorldResidency, chunk_id: &str, z: f64) {
        let mut parts = chunk_id.split('_');
        let cx: f64 = parts.next().unwrap().parse().unwrap();
        let cy: f64 = parts.next().unwrap().parse().unwrap();
        let min_x = cx * 512.0;
        let min_y = cy * 512.0;
        let missing = r.set_viewport(min_x, min_y, min_x + 512.0, min_y + 512.0, z);
        let chunk_path = map_assets()
            .join("everon/objects/chunks")
            .join(format!("{chunk_id}.json.gz"));
        for id in &missing {
            let bytes = fs::read(&chunk_path).unwrap_or_else(|_| {
                fs::read(
                    map_assets()
                        .join("everon/objects/chunks")
                        .join(format!("{id}.json.gz")),
                )
                .unwrap()
            });
            r.ingest_chunk_gz(id, &bytes).unwrap();
        }
        if !missing.is_empty() {
            r.end_apply_frame(0.0);
        }
    }

    fn oracle_badge_count(
        r: &WorldResidency,
        prefab_class: &HashMap<u16, String>,
        prefab_importance: &HashMap<u16, f64>,
        atlas_keys: &HashSet<String>,
        z: f64,
    ) -> usize {
        // T-152.21 — a building emits a badge when the class LOD gate is on OR its per-prefab
        // importanceZoom override fires (deck_zoom ≥ importanceZoom). Below the gate only the
        // tagged landmarks count. (Airfield gating is inert on FIXTURE_CHUNK — no hangar/tower
        // there — so this oracle need not model it; g4 pins that equality at z ≥ 1.)
        let badge_gate = class_visible("buildingBadge", z);
        let building_code = class_code("building");
        let mut n = 0usize;
        for id in &r.draw_ids {
            let Some(chunk) = r.chunks.get(id) else {
                continue;
            };
            let Some(rows) = chunk.rows_by_class.get(&building_code) else {
                continue;
            };
            for &row in rows {
                let row = row as usize;
                let u16k = chunk.prefab_idx[row];
                let Some(cls) = prefab_class.get(&u16k) else {
                    continue;
                };
                let importance_ok = prefab_importance.get(&u16k).is_some_and(|iz| z >= *iz);
                if !badge_gate && !importance_ok {
                    continue;
                }
                let Some(key) = landmark_glyph_icon_key(cls) else {
                    continue;
                };
                if atlas_keys.contains(key) {
                    n += 1;
                }
            }
        }
        n
    }

    fn oracle_landmark_glyph_count_for_chunk(
        r: &WorldResidency,
        chunk_id: &str,
        prefab_class: &HashMap<u16, String>,
        atlas_keys: &HashSet<String>,
        z: f64,
    ) -> usize {
        if !class_visible("buildingBadge", z) {
            return 0;
        }
        let Some(chunk) = r.chunks.get(chunk_id) else {
            return 0;
        };
        let building_code = class_code("building");
        let Some(rows) = chunk.rows_by_class.get(&building_code) else {
            return 0;
        };
        let landmark = ["lighthouse", "castle", "bridge"];
        let mut n = 0usize;
        for &row in rows {
            let row = row as usize;
            let Some(cls) = prefab_class.get(&chunk.prefab_idx[row]) else {
                continue;
            };
            if !landmark.contains(&cls.as_str()) {
                continue;
            }
            let Some(key) = landmark_glyph_icon_key(cls) else {
                continue;
            };
            if atlas_keys.contains(key) {
                n += 1;
            }
        }
        n
    }

    fn badge_glyph_indices(buf: &[u8]) -> Vec<u16> {
        let stride = super::super::glyph_math::ICON_INSTANCE_STRIDE;
        buf.chunks(stride)
            .map(|chunk| u16::from_le_bytes(chunk[14..16].try_into().unwrap()))
            .collect()
    }

    #[test]
    fn g2_building_glyph_lookup_populated() {
        let r = load_everon_residency();
        let n = r.glyph_lookup_len_for_group(2);
        assert!(
            n >= N_MIN_BUILDING_GLYPH_LOOKUP,
            "building glyph lookup {n} < N_min {N_MIN_BUILDING_GLYPH_LOOKUP}"
        );
    }

    /// Guard B (glyph-atlas fix) — the tree lane was never tested on real data (only badges were),
    /// which is how the 28-vs-29 atlas-count bug shipped unseen. Proves the Rust core maps every
    /// real tree prefab to a group-0 glyph and packs a dense forest chunk once keys are registered —
    /// so the "no tree glyphs" defect is provably downstream (the browser atlas-count guard), not core.
    #[test]
    fn tree_glyphs_pack_from_real_everon_data() {
        let mut r = load_everon_residency();
        // Every real Everon tree prefab (iconKey tree-conifer/tree-deciduous) registers as group 0.
        assert_eq!(
            r.glyph_lookup_len_for_group(0),
            51,
            "all real tree prefabs must map to group-0 glyphs"
        );
        // A dense-forest chunk at detail zoom (z=0, tree glyphs on) packs its whole tree census.
        // `drive_fixture_chunk` replicates 16_2's data (6500 trees) into every strict-draw chunk;
        // the z=0 viewport [8192,1024,8704,1536] spans to chunk edges → 4 draw chunks (16_2,17_2,
        // 16_3,17_3) → 4 × 6500 = 26000. Completeness: every visible tree instance packs (none dropped).
        drive_fixture_chunk(&mut r, "16_2", 0.0);
        let packed = r.tree_glyph_count();
        assert!(packed > 0, "forest chunk must pack tree glyphs");
        assert_eq!(
            packed,
            r.exact_tree_count_draw(),
            "every visible tree instance must pack (no silent drops)"
        );
        assert_eq!(
            packed, 26000,
            "4 strict-draw chunks × 6500 replicated fixture trees"
        );
    }

    fn world_glyphs_atlas_keys() -> HashSet<String> {
        let raw = fs::read_to_string(map_assets().join("glyphs/atlas/world-glyphs.json"))
            .expect("world-glyphs.json");
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        v["icons"]
            .as_object()
            .expect("icons object")
            .keys()
            .cloned()
            .collect()
    }

    /// Guard A (glyph-atlas fix) — the two atlas key sources agree, and EVERY glyph key any render
    /// path can request resolves in the browser atlas: every prefab `iconKey` (data path) plus every
    /// `building_icon_key`/`badge_icon_key` classify output. A missing key = a silently-dark glyph
    /// type — the exact class of defect that shipped `29 ≠ 28` unseen. This fails CI loudly instead.
    #[test]
    fn glyph_atlas_covers_every_requested_key_and_sources_agree() {
        let atlas = world_glyphs_atlas_keys();
        // Source parity: the browser atlas (world-glyphs.json) and the manifest source cannot drift.
        let manifest: HashSet<String> = glyph_keys_from_manifest().into_iter().collect();
        assert_eq!(
            atlas, manifest,
            "world-glyphs.json vs manifest.json glyph keys diverged"
        );
        // Data path: every prefab iconKey resolves.
        let raw = super::super::store::bytes_to_json(
            &fs::read(map_assets().join("everon/objects/prefabs.json.gz")).unwrap(),
        )
        .unwrap();
        for row in narrow_prefab_rows(&raw) {
            if let Some(k) = row.icon_key.as_deref() {
                assert!(atlas.contains(k), "prefab iconKey '{k}' missing from atlas");
            }
        }
        // Classify path: every building footprint + badge overlay key resolves.
        for &cls in BUILDING_CLASSES {
            for key in [building_icon_key(cls), badge_icon_key(cls)]
                .into_iter()
                .flatten()
            {
                assert!(
                    atlas.contains(key),
                    "classify key '{key}' (class '{cls}') missing from atlas"
                );
            }
        }
    }

    /// T-152.21 — below the badge band (`buildingBadge` gate at z ≥ 1) only importanceZoom-tagged
    /// landmarks emit; ordinary buildings stay dark. (Was: ALL badges off below 1 — the pre-fix
    /// behavior that left lighthouses as rectangles at the operator's default zoom.)
    #[test]
    fn g3_zoom_gate_below_one_only_importance_landmarks() {
        let mut r = load_everon_residency();
        let keys: HashSet<String> = glyph_keys_from_manifest().into_iter().collect();
        let prefab_class = building_class_by_prefab_u16();
        let prefab_importance = building_importance_by_prefab_u16();
        drive_fixture_chunk(&mut r, FIXTURE_CHUNK, 0.9);
        let rust = r.badge_glyph_count() as usize;
        let oracle = oracle_badge_count(&r, &prefab_class, &prefab_importance, &keys, 0.9);
        assert_eq!(
            rust, oracle,
            "@ z=0.9 Rust badge count must equal the importance-aware oracle"
        );
        // The fix: importanceZoom landmarks (lighthouse/castle) surface below z=1. Equality above
        // already proves ordinary (non-importance) buildings are excluded here — the oracle counts
        // only tagged prefabs when the class gate is off.
        assert!(
            rust > 0,
            "importanceZoom landmarks must emit below the badge band"
        );
    }

    #[test]
    fn g4_class_r_badge_counts_match_oracle() {
        let mut r = load_everon_residency();
        let keys: HashSet<String> = glyph_keys_from_manifest().into_iter().collect();
        let prefab_class = building_class_by_prefab_u16();
        let prefab_importance = building_importance_by_prefab_u16();
        for z in [1.0, 2.0, 3.0] {
            drive_fixture_chunk(&mut r, FIXTURE_CHUNK, z);
            let rust_count = r.badge_glyph_count() as usize;
            let oracle = oracle_badge_count(&r, &prefab_class, &prefab_importance, &keys, z);
            assert_eq!(
                rust_count, oracle,
                "badge count mismatch @ z={z} chunk {FIXTURE_CHUNK}"
            );
        }
    }

    #[test]
    fn g5_landmark_glyph_count_matches_oracle_at_z2() {
        let mut r = load_everon_residency();
        let keys: HashSet<String> = glyph_keys_from_manifest().into_iter().collect();
        let prefab_class = building_class_by_prefab_u16();
        let prefab_importance = building_importance_by_prefab_u16();
        drive_fixture_chunk(&mut r, FIXTURE_CHUNK, 2.0);
        let oracle_lm =
            oracle_landmark_glyph_count_for_chunk(&r, FIXTURE_CHUNK, &prefab_class, &keys, 2.0);
        assert!(
            oracle_lm > 0,
            "fixture chunk must include landmark buildings"
        );
        assert_eq!(
            oracle_lm, 11,
            "pinned fixture {FIXTURE_CHUNK} landmark oracle"
        );
        let lighthouse_idx = r.glyph_idx_for_key("building-lighthouse").unwrap();
        let castle_idx = r.glyph_idx_for_key("building-castle").unwrap();
        let bridge_idx = r.glyph_idx_for_key("building-bridge").unwrap();
        let rust_lm = badge_glyph_indices(&r.world_badge_glyphs())
            .iter()
            .filter(|idx| **idx == lighthouse_idx || **idx == castle_idx || **idx == bridge_idx)
            .count();
        assert!(
            rust_lm >= oracle_lm,
            "composed landmark glyphs {rust_lm} must cover fixture oracle {oracle_lm}"
        );
        assert_eq!(
            r.badge_glyph_count() as usize,
            oracle_badge_count(&r, &prefab_class, &prefab_importance, &keys, 2.0)
        );
    }

    #[test]
    fn g6_lighthouse_instances_emit_building_lighthouse_glyph() {
        let mut r = load_everon_residency();
        let lighthouse_idx = r
            .glyph_idx_for_key("building-lighthouse")
            .expect("building-lighthouse in atlas");
        drive_fixture_chunk(&mut r, FIXTURE_CHUNK, 2.0);
        let indices = badge_glyph_indices(&r.world_badge_glyphs());
        assert!(
            indices.contains(&lighthouse_idx),
            "badge buffer must include building-lighthouse glyph @ z=2"
        );
        assert!(r.badge_glyph_count() > 0);
    }

    /// T-152.21 G2 + G1 — importanceZoom landmarks surface at the default editor zoom (−2) and down
    /// to the override boundary (−4), and vanish just past it. Closes the P1 "white rectangles at
    /// default zoom" complaint: lighthouses/castles read as badges where the operator lives (z=−2).
    #[test]
    fn t152_21_landmark_early_visibility() {
        let mut r = load_everon_residency();
        let keys: HashSet<String> = glyph_keys_from_manifest().into_iter().collect();
        let prefab_class = building_class_by_prefab_u16();
        let prefab_importance = building_importance_by_prefab_u16();
        let lighthouse_idx = r.glyph_idx_for_key("building-lighthouse").unwrap();

        // G2 — default editor zoom (−2): the lighthouse chunk emits landmark badges.
        drive_fixture_chunk(&mut r, FIXTURE_CHUNK, -2.0);
        let n_default = r.badge_glyph_count() as usize;
        assert!(
            n_default > 0,
            "landmarks must emit badges at default zoom −2"
        );
        assert_eq!(
            n_default,
            oracle_badge_count(&r, &prefab_class, &prefab_importance, &keys, -2.0),
            "z=−2 badge count must equal the importance-aware oracle"
        );
        assert!(
            badge_glyph_indices(&r.world_badge_glyphs()).contains(&lighthouse_idx),
            "building-lighthouse glyph present at z=−2"
        );

        // G1 — the override boundary is −4 (classify data): visible AT −4.0, gone at −4.1.
        drive_fixture_chunk(&mut r, FIXTURE_CHUNK, -4.0);
        assert!(
            r.badge_glyph_count() > 0,
            "landmarks visible at the importanceZoom boundary −4.0"
        );
        assert_eq!(
            r.badge_glyph_count() as usize,
            oracle_badge_count(&r, &prefab_class, &prefab_importance, &keys, -4.0)
        );
        drive_fixture_chunk(&mut r, FIXTURE_CHUNK, -4.1);
        assert_eq!(
            r.badge_glyph_count(),
            0,
            "no badges past the −4 override boundary"
        );
        assert_eq!(
            oracle_badge_count(&r, &prefab_class, &prefab_importance, &keys, -4.1),
            0
        );
    }

    /// T-152.21 G4 — fill de-emphasis handoff: while the early landmark glyph draws (below the badge
    /// band) the lighthouse's bright white OBB fill (`[235,235,235,220]`, the literal P1 face) drops
    /// to the neutral footprint fill so the glyph is the coarse-zoom face; at/above the badge band
    /// the bright fill is restored (the badge composes over the normal fill, as before).
    #[test]
    fn t152_21_fill_deemphasis_handoff() {
        let mut r = load_everon_residency();
        let white = 235.0_f32 / 255.0;
        // A fill instance carrying the lighthouse bright-white RGB (stride 10:
        // [x, y, hx, hy, cos, sin, r, g, b, a]). No other class uses this color.
        let has_white_fill = |buf: &[f32]| -> bool {
            buf.chunks_exact(10).any(|c| {
                (c[6] - white).abs() < 1e-3
                    && (c[7] - white).abs() < 1e-3
                    && (c[8] - white).abs() < 1e-3
            })
        };
        // z=−2 (early glyph active): lighthouse fill de-emphasized — no white square.
        drive_fixture_chunk(&mut r, FIXTURE_CHUNK, -2.0);
        assert!(
            !has_white_fill(&r.world_building_fill()),
            "lighthouse white fill must be de-emphasized at z=−2 (glyph is the face)"
        );
        // z=2 (class gate on): bright lighthouse fill restored under the badge overlay.
        drive_fixture_chunk(&mut r, FIXTURE_CHUNK, 2.0);
        assert!(
            has_white_fill(&r.world_building_fill()),
            "lighthouse bright fill present at z≥1 (no de-emphasis above the badge band)"
        );
    }

    #[test]
    fn g1_building_icon_key_covers_normative_classes() {
        use super::super::glyph_math::building_icon_key;
        for &cls in BUILDING_CLASSES {
            let key = building_icon_key(cls).expect(cls);
            assert_eq!(key, format!("building-{cls}"));
        }
    }

    // ---- T-152.15 fence/pier/bridge remediation gates (G2/G3/G5/G6 + bridge casing) ----

    const EVERON_TERRAIN_M: f64 = 12800.0;
    const PIER_CENSUS: u32 = 2299;
    const BRIDGE_CENSUS: usize = 144;

    /// Pin + ingest every real chunk of the island once (each cell's OWN file — unlike
    /// `drive_fixture_chunk`, which replicates one fixture). No per-chunk sum ⇒ no LRU/extra-ring
    /// double-count: one residency holds the whole island, so the strip counts are exact.
    fn drive_full_island(r: &mut WorldResidency, z: f64) {
        let missing = r.set_viewport(0.0, 0.0, EVERON_TERRAIN_M, EVERON_TERRAIN_M, z);
        let dir = map_assets().join("everon/objects/chunks");
        for id in &missing {
            match fs::read(dir.join(format!("{id}.json.gz"))) {
                Ok(bytes) => {
                    r.ingest_chunk_gz(id, &bytes).unwrap();
                }
                Err(_) => r.note_undelivered(id),
            }
        }
        r.end_apply_frame(0.0);
    }

    /// Bridge instance count over the pinned island — independent of the rail-compose path
    /// (different code) so the G5 `rails == 2 × bridges` check is not tautological.
    fn island_bridge_count(r: &WorldResidency) -> usize {
        let building_code = class_code("building");
        let mut ids = r.pinned_ids.clone();
        ids.sort();
        let mut n = 0usize;
        for id in &ids {
            let Some(chunk) = r.chunks.get(id) else {
                continue;
            };
            let Some(rows) = chunk.rows_by_class.get(&building_code) else {
                continue;
            };
            for &row in rows {
                let row = row as usize;
                if let Some(info) = r.building_by_u16.get(&chunk.prefab_idx[row])
                    && info.building_class == "bridge"
                {
                    n += 1;
                }
            }
        }
        n
    }

    /// Count building-fill instances (10 f32 each) whose RGBA equals `rgba`.
    fn fill_instances_with_color(fill: &[f32], rgba: [u8; 4]) -> usize {
        let want = norm(rgba);
        fill.chunks_exact(10)
            .filter(|inst| {
                (inst[6] - want[0]).abs() < 1e-6
                    && (inst[7] - want[1]).abs() < 1e-6
                    && (inst[8] - want[2]).abs() < 1e-6
                    && (inst[9] - want[3]).abs() < 1e-6
            })
            .count()
    }

    /// G2 — strip long axis ≡ fill OBB long axis within 0.5° over EVERY real fence + pier/dock
    /// prefab × sample yaws {0,37,90,123}. Anti-vacuous: asserts ≥ 255 fence prefabs × 4 yaws.
    #[test]
    fn t152_15_g2_orientation_parity_all_prefabs() {
        let everon = map_assets().join("everon");
        let raw = super::super::store::bytes_to_json(
            &fs::read(everon.join("objects/prefabs.json.gz")).unwrap(),
        )
        .unwrap();
        let fences = fence_prefab_lookup(&raw);
        let buildings = building_prefab_lookup(&raw);
        let mut samples: Vec<(f64, f64)> = fences.values().map(|f| (f.half_x, f.half_y)).collect();
        let fence_n = samples.len();
        for info in buildings.values() {
            if info.building_class == "pier" || info.building_class == "dock" {
                samples.push((info.half_x, info.half_y));
            }
        }
        let mut checked = 0usize;
        let mut worst = 0.0f64;
        for (hx, hy) in &samples {
            for yaw in [0.0f64, 37.0, 90.0, 123.0] {
                let [p0, p1] = super::super::obb_long_axis_endpoints(0.0, 0.0, *hx, *hy, yaw);
                let strip_ang = (p1[1] - p0[1]).atan2(p1[0] - p0[0]).to_degrees();
                let c = obb_corners(0.0, 0.0, *hx, *hy, yaw);
                let (e0x, e0y) = (c[1][0] - c[0][0], c[1][1] - c[0][1]); // len 2·hx
                let (e1x, e1y) = (c[2][0] - c[1][0], c[2][1] - c[1][1]); // len 2·hy
                let (fx, fy) = if e0x.hypot(e0y) >= e1x.hypot(e1y) {
                    (e0x, e0y)
                } else {
                    (e1x, e1y)
                };
                let fill_ang = fy.atan2(fx).to_degrees();
                let d = {
                    let x = (strip_ang - fill_ang).abs() % 180.0;
                    x.min(180.0 - x)
                };
                assert!(d <= 0.5, "parity {d}° for hx={hx} hy={hy} yaw={yaw}");
                worst = worst.max(d);
                checked += 1;
            }
        }
        assert!(fence_n >= 255, "expected ≥255 fence prefabs, got {fence_n}");
        assert!(
            checked >= 255 * 4,
            "parity gate must be non-vacuous, only {checked} checks"
        );
        assert!(worst <= 0.5, "worst-case parity {worst}° exceeds 0.5°");
    }

    /// G3 (pier census) + G5 (bridge rails) + bridge casing + G6 (toggle decoupling), one island.
    #[test]
    fn t152_15_pier_census_rails_casing_and_decoupling() {
        let mut r = load_everon_residency();
        drive_full_island(&mut r, 1.5); // z ≥ fence(1.5), pier(−1.0), building(−2.5): all lanes on

        // G3 — every pier/dock draws (compose emits unconditionally). Anti-vacuous: > 0.
        let piers = r.pier_strip_segment_count();
        assert!(piers > 0, "G3 anti-vacuous: pier census must be > 0");
        assert!(
            piers >= (f64::from(PIER_CENSUS) * 0.99).ceil() as u32,
            "G3: pier census {piers} < 0.99 × {PIER_CENSUS}"
        );
        assert_eq!(piers, PIER_CENSUS, "G3: exact pier census");

        // G5 — 2 synthetic rails per bridge instance (Path A), by construction.
        let bridges = island_bridge_count(&r);
        assert!(bridges > 0, "G5 anti-vacuous: island must have bridges");
        assert_eq!(bridges, BRIDGE_CENSUS, "bridge instance census");
        assert_eq!(
            r.bridge_rail_strip_count(),
            2 * bridges as u32,
            "G5: every bridge emits exactly 2 rail strips"
        );

        // Bridge casing (Q6) — each bridge contributes one deck fill + one casing fill.
        let fill = r.world_building_fill();
        assert_eq!(
            fill_instances_with_color(&fill, BRIDGE_DECK_RGBA),
            bridges,
            "one warm-deck fill per bridge"
        );
        assert_eq!(
            fill_instances_with_color(&fill, BRIDGE_CASING_RGBA),
            bridges,
            "one casing rim per bridge"
        );

        // Fences on at z=1.5.
        let fences_on = r.fence_strip_segment_count();
        assert!(fences_on > 0, "fences must draw at z=1.5");

        // G6 — Fences OFF ⇒ 0 fence strips; piers + rails unaffected.
        r.set_fences_toggle(false);
        assert_eq!(r.fence_strip_segment_count(), 0, "G6: fences off ⇒ 0 fence");
        assert_eq!(
            r.pier_strip_segment_count(),
            piers,
            "G6: piers unaffected by fences"
        );
        assert_eq!(
            r.bridge_rail_strip_count(),
            2 * bridges as u32,
            "G6: rails unaffected by fences"
        );
        r.set_fences_toggle(true);
        assert_eq!(r.fence_strip_segment_count(), fences_on, "fences restored");

        // G6 — Buildings OFF ⇒ 0 pier strips + 0 rails; fences unaffected.
        r.set_glyph_toggles(true, false, false);
        assert_eq!(
            r.pier_strip_segment_count(),
            0,
            "G6: buildings off ⇒ 0 pier"
        );
        assert_eq!(
            r.bridge_rail_strip_count(),
            0,
            "G6: buildings off ⇒ 0 rails"
        );
        assert_eq!(
            r.fence_strip_segment_count(),
            fences_on,
            "G6: fences unaffected by buildings toggle"
        );
    }
}
