//! T-166 W4 — TBDD forest mass stream (React `useWgpuForestMass` port).

use std::collections::{HashMap, HashSet, VecDeque};

use map_engine_core::geometry::forest_mass::{
    forest_fill_alpha, forest_mass_from_corners, DENSITY_ISO,
};
use map_engine_core::geometry::tbdd::decode_tbdd;
use map_engine_core::geometry::vector_compose::{compose_forest_mesh, retint_fill_alpha};
use map_engine_core::world::{chunk_ids_for_viewport, class_visible, TerrainSizeM};

use crate::select_tool::EngineHandle;

use super::bridge::{publish_engine, BridgeHandle};
use super::fetch::fetch_bytes;

const ROLE_FOREST_FILL: u32 = 5;
const ROLE_FOREST_OUTLINE: u32 = 6;
const CHUNK_SIZE_M: f64 = 512.0;
const FETCH_CONCURRENCY: usize = 12;
const LRU_MIN: usize = 64;
const TERRAIN: TerrainSizeM = TerrainSizeM {
    width: 12_800.0,
    height: 12_800.0,
};

struct Composed {
    fill_pos: Vec<f32>,
    fill_col: Vec<f32>,
    fill_idx: Vec<u32>,
    fill_n: u32,
    outline: Vec<f32>,
    outline_n: u32,
}

pub struct ForestMassHost {
    asset_base: String,
    ready: bool,
    cache: HashMap<String, Option<Composed>>,
    lru: VecDeque<String>,
    last_key: String,
    /// What the last `push_composite` actually uploaded: (ids key, alpha bits, chunks present,
    /// fill on, outline on). Identical state → skip the concat + re-upload entirely — the boot
    /// drain (12 passes) and each settle (6 passes) were re-uploading an unchanged mesh every
    /// pass (T-172 H2).
    pushed: (String, u64, usize, bool, bool),
}

impl ForestMassHost {
    pub fn new() -> Self {
        Self {
            asset_base: String::new(),
            ready: false,
            cache: HashMap::new(),
            lru: VecDeque::new(),
            last_key: String::new(),
            pushed: (String::new(), u64::MAX, usize::MAX, false, false),
        }
    }

    pub fn init(&mut self, terrain: &str) {
        self.asset_base = format!("/map-assets/{terrain}");
        self.ready = true;
    }

    /// One forest-mass settle pass. Returns whether it did real work (fetched or re-uploaded) so
    /// `flush_viewport` can break its multi-pass loop when idle (T-173 P2).
    pub async fn run_viewport(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) -> bool {
        if !self.ready {
            return false;
        }
        let (bounds, zoom) = {
            let g = engine.borrow();
            let Some(e) = g.as_ref() else {
                return false;
            };
            (e.visible_bounds(), e.zoom())
        };
        if bounds.len() < 4 {
            return false;
        }
        let ids = chunk_ids_for_viewport(
            [bounds[0], bounds[1], bounds[2], bounds[3]],
            TERRAIN,
            CHUNK_SIZE_M,
            0,
        );
        let key = ids.join(",");
        let mut fetched = false;
        if key != self.last_key {
            self.last_key = key;
            self.touch_lru(&ids);
            let missing: Vec<String> = ids
                .iter()
                .filter(|id| !self.cache.contains_key(id.as_str()))
                .cloned()
                .collect();
            if !missing.is_empty() {
                fetched = true;
                self.fetch_missing(missing).await;
            }
        }
        let pushed = self.push_composite(engine, bridge, zoom, &ids);
        fetched || pushed
    }

    fn touch_lru(&mut self, ids: &[String]) {
        let pinned: HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
        for id in ids {
            self.lru.retain(|x| x != id);
            self.lru.push_back(id.clone());
        }
        let cap = LRU_MIN.max(ids.len() * 3);
        while self.lru.len() > cap {
            if let Some(evict) = self.lru.pop_front() {
                if !pinned.contains(evict.as_str()) {
                    self.cache.remove(&evict);
                } else {
                    self.lru.push_back(evict);
                    break;
                }
            } else {
                break;
            }
        }
    }

    async fn fetch_missing(&mut self, ids: Vec<String>) {
        let base = self.asset_base.clone();
        for batch in ids.chunks(FETCH_CONCURRENCY) {
            for id in batch {
                let url = format!("{base}/objects/density/{id}.bin");
                let bytes = fetch_bytes(&url).await;
                let composed = bytes.and_then(|b| compose_chunk(id, &b));
                self.cache.insert(id.clone(), composed);
            }
        }
    }

    fn push_composite(
        &mut self,
        engine: &EngineHandle,
        bridge: &BridgeHandle,
        zoom: f64,
        ids: &[String],
    ) -> bool {
        let fill_on = class_visible("forestFill", zoom);
        let outline_on = class_visible("forestOutline", zoom);
        let alpha = forest_fill_alpha(zoom);
        if !fill_on && !outline_on {
            // Already hidden → nothing to do (avoids re-clearing every idle pass).
            if self.pushed == (String::new(), u64::MAX, usize::MAX, false, false) {
                return false;
            }
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.clear_vector_lane(ROLE_FOREST_FILL);
                e.clear_vector_lane(ROLE_FOREST_OUTLINE);
            }
            // Re-arm the memo so the next visible push uploads.
            self.pushed = (String::new(), u64::MAX, usize::MAX, false, false);
            return true;
        }
        // Skip identical re-uploads (T-172 H2): same viewport chunk set, same alpha band, same
        // number of resolved chunks, same class flags → the concatenated mesh is byte-identical.
        let present = ids.iter().filter(|id| self.cache.contains_key(*id)).count();
        let state = (
            self.last_key.clone(),
            alpha.to_bits(),
            present,
            fill_on,
            outline_on,
        );
        if state == self.pushed {
            return false;
        }
        let mut pos = Vec::new();
        let mut col = Vec::new();
        let mut idx = Vec::new();
        let mut base_v = 0u32;
        let mut poly_n = 0u32;
        let mut outline = Vec::new();
        let mut outline_n = 0u32;
        for id in ids {
            let Some(Some(c)) = self.cache.get(id) else {
                continue;
            };
            if fill_on && alpha > 0.0 && c.fill_n > 0 {
                pos.extend_from_slice(&c.fill_pos);
                col.extend_from_slice(&c.fill_col);
                for &i in &c.fill_idx {
                    idx.push(base_v + i);
                }
                base_v += (c.fill_pos.len() / 2) as u32;
                poly_n += c.fill_n;
            }
            if outline_on && c.outline_n > 0 {
                outline.extend_from_slice(&c.outline);
                outline_n += c.outline_n;
            }
        }
        // The cached chunk meshes are baked at alpha 1.0; tint the concatenated copy to the zoom
        // band so the fill blends translucent over the basemap (T-172 B3).
        #[allow(clippy::cast_possible_truncation)]
        retint_fill_alpha(&mut col, alpha as f32);
        if let Some(e) = engine.borrow_mut().as_mut() {
            if fill_on && poly_n > 0 {
                e.upload_polygon_mesh(ROLE_FOREST_FILL, &pos, &col, &idx, poly_n, true);
            } else {
                e.clear_vector_lane(ROLE_FOREST_FILL);
            }
            if outline_on && outline_n > 0 {
                e.upload_hairline_segments(ROLE_FOREST_OUTLINE, &outline, outline_n, true);
            } else {
                e.clear_vector_lane(ROLE_FOREST_OUTLINE);
            }
            publish_engine(bridge, e);
        }
        self.pushed = state;
        true
    }
}

impl Default for ForestMassHost {
    fn default() -> Self {
        Self::new()
    }
}

fn compose_chunk(id: &str, bytes: &[u8]) -> Option<Composed> {
    let grid = decode_tbdd(bytes).ok()?;
    let tree = grid.channels.first()?;
    let (cx, cy) = parse_xy(id)?;
    let origin_x = f64::from(cx) * CHUNK_SIZE_M;
    let origin_y = f64::from(cy) * CHUNK_SIZE_M;
    let geo = forest_mass_from_corners(
        tree,
        grid.cols as usize,
        grid.rows as usize,
        origin_x,
        origin_y,
        f64::from(grid.cell_m),
        DENSITY_ISO,
    );
    let (fill, outline) = compose_forest_mesh(&geo, 1.0);
    Some(Composed {
        fill_pos: fill.positions,
        fill_col: fill.colors,
        fill_idx: fill.indices,
        fill_n: fill.polygon_count,
        outline: outline.verts,
        outline_n: outline.segment_count,
    })
}

fn parse_xy(id: &str) -> Option<(u32, u32)> {
    let (a, b) = id.split_once('_')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}
