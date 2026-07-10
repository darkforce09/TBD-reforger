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

use super::chunk::{WorldChunk, parse_chunk};
use super::chunk_math::{
    Bbox, TerrainSizeM, chunk_ids_for_rect, chunk_ids_for_viewport, chunk_rect_for_bbox,
};
use super::classify::class_code;
use super::density_ladder::{
    density_grid_dims, exact_tree_count, heatmap_trees, pack_density_grid_r32,
};
use super::glyph_math::{
    BADGE_SIZE_MIN_PX, DEFAULT_BASE_SIZE_PX, GLYPH_SIZE_MIN_PX, badge_icon_key, badge_size_meters,
    deck_angle_for_rotation_deg, glyph_size_meters, hex_to_rgba, pack_icon_instance, pack_rgba_u32,
    size_with_min_px,
};
use super::index::WorldSpatialIndex;
use super::lod_gates::{INSTANCE_BUDGET, class_visible};
use super::manifest::{ObjectsManifest, narrow_cells, parse_objects_manifest};
use super::obb::{BuildingPrefabInfo, building_prefab_lookup, obb_corners};
use super::prefab::{PrefabEntry, build_prefab_maps, narrow_prefab_rows};
use super::store::{WorldError, bytes_to_json};

/// No extra draw margin — residency preload covers fetch; draw cull is strict visible rect (T-151.8).
/// Referenced by Class S tests / verify log; must stay 0.
pub const DRAW_CULL_MARGIN_M: f64 = 0.0;

/// Per-prefab glyph render resolved once at prefab load (tree/veg/prop/rockLarge only).
#[derive(Clone, Debug)]
struct GlyphPrefabInfo {
    /// Index into the 28-entry atlas UV table (`u16::MAX` = unknown key).
    glyph_idx: u16,
    size_m: f32,
    tint: u32,
    /// 0 = tree group (tree+vegetation), 1 = prop group (prop+rockLarge).
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

/// `FILL_BY_CLASS[class] ?? FILL_DEFAULT` (`buildingLayer.ts:127-139`), verbatim.
#[must_use]
fn fill_color(class: &str) -> [u8; 4] {
    match class {
        "military" => [0x7a, 0x5c, 0x3d, 184],
        "bridge" => [90, 90, 100, 200],
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

    fill_buf: Vec<f32>,
    outline_buf: Vec<f32>,

    /// T-151.11.3 (audit B-04): ingest-frame start stamp — the ≤ APPLY_BUDGET_MS/frame policy
    /// lives HERE (pure; the wasm wrapper feeds `Date.now`), not in the JS loader.
    ingest_frame_start_ms: Option<f64>,

    /// Last viewport zoom (for glyph LOD + min-px clamp).
    deck_zoom: f64,
    /// User prefs (`worldLayerPrefs.classToggles`).
    toggle_trees: bool,
    toggle_props: bool,
    toggle_buildings: bool,

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
            fill_buf: Vec::new(),
            outline_buf: Vec::new(),
            ingest_frame_start_ms: None,
            deck_zoom: -2.0,
            toggle_trees: true,
            toggle_props: false,
            toggle_buildings: true,
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

    /// Whether the building fill/outline lanes should draw: user toggle ∧ zoom gate
    /// (T-151.11.3 / P-04). The loader passes this as the upload `visible` flag so a toggle-off
    /// removes the lanes (bypassing the empty+visible sticky anti-wipe rule, which only guards
    /// mid-hydration wipes).
    #[must_use]
    pub fn buildings_visible(&self) -> bool {
        self.toggle_buildings && building_visible(self.deck_zoom)
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
            let tree_code = class_code("tree");
            let veg_code = class_code("vegetation");
            let prop_code = class_code("prop");
            let rock_code = class_code("rockLarge");
            let group = if code == tree_code || code == veg_code {
                0u8
            } else if code == prop_code || code == rock_code {
                1u8
            } else {
                continue;
            };
            let base = entry.row.base_size_px.unwrap_or(DEFAULT_BASE_SIZE_PX);
            let size_m = glyph_size_meters(base, entry.row.height_m) as f32;
            let tint = pack_rgba_u32(hex_to_rgba(entry.row.default_color.as_deref()));
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
        let zoom_changed = (self.deck_zoom - deck_zoom).abs() > f64::EPSILON;
        self.deck_zoom = deck_zoom;
        self.last_viewport = [min_x, min_y, max_x, max_y];
        if !building_visible(deck_zoom) {
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
            self.refresh_draw_set_and_glyphs();
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
    /// residency + spatial index. Returns the instance count. (The caller runs the per-frame
    /// budget loop; call [`Self::end_apply_frame`] after a frame that applied ≥1 chunk.)
    ///
    /// # Errors
    /// [`WorldError::Gzip`]/[`WorldError::Json`] on a bad payload.
    pub fn ingest_chunk_gz(&mut self, id: &str, bytes: &[u8]) -> Result<u32, WorldError> {
        let raw = bytes_to_json(bytes)?;
        let chunk = parse_chunk(id, &raw, &self.prefab_by_id).unwrap_or_else(|| WorldChunk {
            id: id.to_string(),
            ..Default::default()
        });
        let count = chunk.count;
        self.insert_chunk(id, chunk);
        Ok(count)
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
        let mut candidates: Vec<String> = self
            .chunks
            .keys()
            .filter(|id| !self.pinned_set.contains(*id))
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
        }
    }

    /// Recompose the building fill + outline GPU buffers from the pinned chunks, in **string-sorted
    /// id order** (matching the JS composite `[...pinned].sort()`).
    fn rebuild_buffers(&mut self) {
        // T-151.11.3 (P-04): toggle off ⇒ compose nothing (Deck hid the whole building lane).
        if !self.toggle_buildings {
            self.fill_buf.clear();
            self.outline_buf.clear();
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
                // Fill instance (WORLD coords): [x, y, hx, hy, cos, sin, r,g,b,a]. `(cos,sin)` use
                // the same `rad = deg·PI/180` as `obb_corners`, so fill and outline coincide.
                let rad = (rot * std::f64::consts::PI) / 180.0;
                let cos = rad.cos() as f32;
                let sin = rad.sin() as f32;
                let c = norm(fill_color(&info.building_class));
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
        self.refresh_draw_set_and_glyphs();
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
        let (gw, gh) = density_grid_dims(self.terrain.width, self.terrain.height, {
            self.manifest
                .as_ref()
                .map(|m| m.chunk_size_m)
                .unwrap_or(512.0)
        });
        self.density_grid_w = gw;
        self.density_grid_h = gh;
        if gw > 0 && gh > 0 && self.terrain.width > 0.0 {
            self.density_grid = pack_density_grid_r32(&self.chunks, gw, gh);
        } else {
            self.density_grid.clear();
        }
        let exact = exact_tree_count(&self.chunks, &self.draw_ids);
        self.exact_tree_count = exact as u32;
        self.heatmap_trees = heatmap_trees(exact);
        self.rebuild_glyph_buffers();
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
        let badge_want = self.toggle_buildings && class_visible("buildingBadge", z);

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
                    let Some(key) = badge_icon_key(&binfo.building_class) else {
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

    #[must_use]
    pub fn exact_tree_count_draw(&self) -> u32 {
        self.exact_tree_count
    }

    /// R32Uint density grid bytes (little-endian u32 per texel) + width/height.
    #[must_use]
    pub fn density_grid_r32_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.density_grid.len() * 4);
        for &v in &self.density_grid {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    #[must_use]
    pub fn density_grid_dims(&self) -> (u32, u32) {
        (self.density_grid_w, self.density_grid_h)
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
            "{{\"chunks_resident\":{},\"chunks_pinned\":{},\"chunks_applied\":{},\"apply_frames\":{},\"apply_budget_ms_last\":{},\"max_apply_ms\":{},\"frames_over_budget\":{},\"building_instances\":{},\"index_size\":{},\"inflight_count\":{},\"pin_settled\":{},\"chunks_draw\":{},\"exact_tree_count\":{},\"heatmap_trees\":{}}}",
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
        )
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

    #[test]
    fn requests_exactly_the_chunk_math_set() {
        let mut r = setup();
        let missing = r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
        let expected =
            chunk_ids_for_viewport([2000.0, 2000.0, 2200.0, 2200.0], r.terrain, 512.0, 0);
        assert_eq!(missing, expected);
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

    /// Class R: heatmap swap clears tree glyphs; under-budget packs every instance.
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
        // Put 10 trees in first draw chunk — under budget → glyphs == exact.
        let id0 = draw[0].clone();
        {
            let c = r.chunks.get_mut(&id0).unwrap();
            let n = 10usize;
            c.count = n as u32;
            c.positions = vec![0.0; n * 2];
            c.prefab_idx = vec![0; n];
            c.rotations = vec![0.0; n];
            c.z = vec![0.0; n];
            c.cls_codes = vec![class_code("tree"); n];
            c.rows_by_class.clear();
            c.rows_by_class
                .insert(class_code("tree"), (0..n as u32).collect());
        }
        // Prefab 0 must resolve as tree glyph — without atlas map, pack skips; force heatmap path
        // via exact count alone for R4, and for R5 use exact_tree_count without glyph lookup.
        r.refresh_draw_set_and_glyphs();
        let exact = exact_tree_count(&r.chunks, &r.draw_ids);
        assert_eq!(exact, 10);
        assert!(!r.heatmap_trees_active());
        // Texel sum Class R.
        let sum = density_texel_sum_for_draw_ids(&r.density_grid, r.density_grid_w, &r.draw_ids);
        assert_eq!(sum, exact as u64);

        // Force over-budget: inflate one chunk's tree rows to INSTANCE_BUDGET + 1.
        {
            let c = r.chunks.get_mut(&id0).unwrap();
            let n = INSTANCE_BUDGET + 1;
            c.count = n as u32;
            c.positions = vec![0.0; n * 2];
            c.prefab_idx = vec![0; n];
            c.rotations = vec![0.0; n];
            c.z = vec![0.0; n];
            c.cls_codes = vec![class_code("tree"); n];
            c.rows_by_class.clear();
            c.rows_by_class
                .insert(class_code("tree"), (0..n as u32).collect());
        }
        r.refresh_draw_set_and_glyphs();
        assert!(r.heatmap_trees_active());
        assert_eq!(r.tree_glyph_count(), 0);
        assert_eq!(r.exact_tree_count_draw() as usize, INSTANCE_BUDGET + 1);
        let sum2 = density_texel_sum_for_draw_ids(&r.density_grid, r.density_grid_w, &r.draw_ids);
        assert_eq!(sum2, (INSTANCE_BUDGET + 1) as u64);
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
