//! T-166 W2–W5 — WorldStore + WorldResidency host (React `wgpuWorldLoader` port).

use std::collections::VecDeque;

use map_engine_core::geometry::vector_compose::{
    compose_landcover_mesh, compose_roads_mesh, LandcoverInput, RoadInput,
};
use map_engine_core::world::{WorldResidency, WorldStore};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::select_tool::EngineHandle;

use super::bridge::{publish_engine, BridgeHandle};
use super::fetch::{fetch_bytes, fetch_text};

const ROLE_LANDCOVER: u32 = 1;
const ROLE_ROADS_CASING: u32 = 3;
const ROLE_ROADS: u32 = 4;
const FETCH_CONCURRENCY: usize = 12;
const ATLAS_WEBP: &str = "/map-assets/glyphs/atlas/world-glyphs.webp";
const ATLAS_JSON: &str = "/map-assets/glyphs/atlas/world-glyphs.json";

struct PendingChunk {
    id: String,
    bytes: Option<Vec<u8>>,
}

struct AtlasUpload {
    rgba: Vec<u8>,
    w: u32,
    h: u32,
    uv: Vec<f32>,
    keys: Vec<String>,
}

pub struct WorldHost {
    residency: WorldResidency,
    store: WorldStore,
    asset_base: String,
    chunks_path: String,
    ready: bool,
    pending: VecDeque<PendingChunk>,
    atlas: Option<AtlasUpload>,
    atlas_uploaded: bool,
    roads_loaded: bool,
    landcover_ready: bool,
    last_road_band: String,
}

impl WorldHost {
    pub fn new() -> Self {
        Self {
            residency: WorldResidency::new(),
            store: WorldStore::new(),
            asset_base: String::new(),
            chunks_path: String::new(),
            ready: false,
            pending: VecDeque::new(),
            atlas: None,
            atlas_uploaded: false,
            roads_loaded: false,
            landcover_ready: false,
            last_road_band: String::new(),
        }
    }

    pub async fn init(&mut self, terrain: &str) -> bool {
        let base = format!("/map-assets/{terrain}");
        let Some(manifest) = fetch_text(&format!("{base}/manifest.json")).await else {
            return false;
        };
        if self.residency.load_manifest_json(&manifest).is_err() {
            return false;
        }
        if self.store.load_manifest_json(&manifest).is_err() {
            return false;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&manifest) else {
            return false;
        };
        let Some(objects) = v.get("objects") else {
            return false;
        };
        let Some(prefabs) = objects.get("prefabsPath").and_then(|x| x.as_str()) else {
            return false;
        };
        let Some(chunks) = objects.get("chunksPath").and_then(|x| x.as_str()) else {
            return false;
        };
        let roads = objects
            .get("roadsPath")
            .and_then(|x| x.as_str())
            .unwrap_or("objects/roads.json.gz");
        let regions = objects
            .get("regionsPath")
            .and_then(|x| x.as_str())
            .unwrap_or("objects/forest-regions.json.gz");
        self.asset_base = base.clone();
        self.chunks_path = chunks.to_string();

        if let Some(bytes) = fetch_bytes(&format!("{base}/{prefabs}")).await {
            let _ = self.residency.load_prefabs_gz(&bytes);
        }
        if let Some(idx) = fetch_text(&format!("{base}/{chunks}/manifest.json")).await {
            let _ = self.residency.load_chunk_index_json(&idx);
        }
        self.atlas = load_glyph_atlas().await;
        if let Some(bytes) = fetch_bytes(&format!("{base}/{roads}")).await {
            if self.store.load_roads_gz(&bytes).is_ok() {
                self.roads_loaded = true;
            }
        }
        if let Some(bytes) = fetch_bytes(&format!("{base}/{regions}")).await {
            if self.store.load_forest_regions_gz(&bytes).is_ok() {
                self.landcover_ready = true;
            }
        }
        self.ready = true;
        true
    }

    pub async fn run_viewport(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) {
        if !self.ready {
            return;
        }
        self.ensure_atlas(engine, bridge);
        let (bounds, zoom) = {
            let g = engine.borrow();
            let Some(e) = g.as_ref() else {
                return;
            };
            (e.visible_bounds(), e.zoom())
        };
        if bounds.len() < 4 {
            return;
        }
        self.push_roads(engine, zoom);
        self.push_landcover(engine);
        let mut missing = self
            .residency
            .set_viewport(bounds[0], bounds[1], bounds[2], bounds[3], zoom);
        // Recover empty stubs / stuck inflight for the tree-glyph band (soft-fail must not
        // permanently cache count=0 over a real forest chunk like 12_12).
        if zoom >= 0.0 {
            let mut dirty = false;
            for id in self.residency.draw_ids().to_vec() {
                match self.residency.resident_instance_count(&id) {
                    Some(0) | None => {
                        self.residency.invalidate_chunk(&id);
                        dirty = true;
                    }
                    Some(_) => {}
                }
            }
            if dirty {
                missing = self
                    .residency
                    .set_viewport(bounds[0], bounds[1], bounds[2], bounds[3], zoom);
            }
        }
        if !missing.is_empty() {
            self.fetch_and_queue(missing).await;
        }
        self.drain(engine, bridge);
        self.push_to_engine(engine, bridge);
    }

    fn ensure_atlas(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) {
        if self.atlas_uploaded {
            return;
        }
        let Some(atlas) = self.atlas.take() else {
            return;
        };
        {
            let mut g = engine.borrow_mut();
            let Some(e) = g.as_mut() else {
                self.atlas = Some(atlas);
                return;
            };
            if e.upload_glyph_atlas(&atlas.rgba, atlas.w, atlas.h, &atlas.uv)
                .is_ok()
            {
                self.residency.set_glyph_key_map(&atlas.keys);
                self.atlas_uploaded = true;
                bridge.borrow_mut().glyph_atlas = true;
            } else {
                self.atlas = Some(atlas);
            }
        }
    }

    fn push_roads(&mut self, engine: &EngineHandle, zoom: f64) {
        if !self.roads_loaded {
            return;
        }
        let band = format!("{}", (zoom * 2.0).round() as i64);
        if band == self.last_road_band {
            return;
        }
        self.last_road_band = band;
        let inputs: Vec<RoadInput<'_>> = self
            .store
            .roads
            .iter()
            .map(|r| RoadInput {
                road_class: r.road_class.as_str(),
                points: r.points.as_slice(),
                width_m: r.width_m,
            })
            .collect();
        let mesh = compose_roads_mesh(&inputs, zoom, true);
        let vis = mesh.segment_count > 0;
        if let Some(e) = engine.borrow_mut().as_mut() {
            e.upload_strip_tris(ROLE_ROADS_CASING, &mesh.casing, mesh.segment_count, vis);
            e.upload_strip_tris(ROLE_ROADS, &mesh.centerline, mesh.segment_count, vis);
        }
    }

    fn push_landcover(&mut self, engine: &EngineHandle) {
        if !self.landcover_ready {
            return;
        }
        let vis = self.residency.forest_fill_effective();
        if !vis {
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.clear_vector_lane(ROLE_LANDCOVER);
            }
            return;
        }
        let inputs: Vec<LandcoverInput<'_>> = self
            .store
            .regions
            .iter()
            .map(|r| LandcoverInput {
                kind: r.kind.as_str(),
                rings: r.polygon.as_slice(),
            })
            .collect();
        let mesh = compose_landcover_mesh(&inputs);
        if let Some(e) = engine.borrow_mut().as_mut() {
            e.upload_polygon_mesh(
                ROLE_LANDCOVER,
                &mesh.positions,
                &mesh.colors,
                &mesh.indices,
                mesh.polygon_count,
                true,
            );
        }
    }

    async fn fetch_and_queue(&mut self, ids: Vec<String>) {
        // Do NOT clear_inflight here — set_viewport already marked these ids. Clearing would
        // drop the pin-settled contract and race with a concurrent settle.
        self.residency.mark_inflight(&ids);
        let base = self.asset_base.clone();
        let chunks = self.chunks_path.clone();
        let mut fetched = Vec::with_capacity(ids.len());
        // Sequential batches of FETCH_CONCURRENCY (wasm-friendly).
        for batch in ids.chunks(FETCH_CONCURRENCY) {
            for id in batch {
                let url = format!("{base}/{chunks}/{id}.json.gz");
                let bytes = fetch_bytes(&url).await;
                fetched.push(PendingChunk {
                    id: id.clone(),
                    bytes,
                });
            }
        }
        // Newest fetches first — the 4ms ingest budget only applies ~1 heavy chunk/frame, so
        // appending behind a z=-2 backlog starved the z≥0 tree-glyph probe (12_12 never ingested).
        for item in fetched.into_iter().rev() {
            self.pending.push_front(item);
        }
    }

    fn drain(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) {
        let now = js_sys::Date::now();
        self.residency.begin_ingest_frame_at(now);
        let mut applied = 0u32;
        // Allow several chunks per settle: the locked 4ms budget is a soft per-frame cap for pan
        // streaming; a camera-settle / smoke probe must clear the viewport's missing set.
        const MAX_PER_SETTLE: u32 = 24;
        while !self.pending.is_empty() && applied < MAX_PER_SETTLE {
            let Some(next) = self.pending.pop_front() else {
                break;
            };
            match next.bytes {
                Some(bytes) => match self.residency.ingest_chunk_gz(&next.id, &bytes) {
                    Ok(n) => {
                        if n == 0 {
                            // Empty parse — do not keep a stub; retry next settle.
                            self.residency.invalidate_chunk(&next.id);
                        }
                    }
                    Err(_) => {
                        // Transient parse/gzip failure — release inflight so the next settle retries.
                        // Do NOT note_undelivered (that caches an empty stub forever).
                        self.residency.release_inflight(&next.id);
                    }
                },
                None => {
                    // Transient HTTP failure — retry next settle; never cache empty.
                    self.residency.release_inflight(&next.id);
                }
            }
            applied += 1;
        }
        if applied > 0 {
            self.residency.end_ingest_frame_at(js_sys::Date::now());
            self.push_to_engine(engine, bridge);
        }
    }

    fn push_to_engine(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) {
        let fill = self.residency.world_building_fill();
        let outline = self.residency.world_building_outline();
        let stats = self.residency.stats_json();
        let Ok(rstats) = serde_json::from_str::<serde_json::Value>(&stats) else {
            return;
        };
        let inflight = rstats
            .get("inflight_count")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        let pin_settled = rstats
            .get("pin_settled")
            .and_then(|x| x.as_bool())
            .unwrap_or(false);
        // Sticky empty mid-hydration for *buildings* only — never skip glyph lanes when the
        // viewport is tree-only (zoom-in probe at forest center has fill=[] while trees pack).
        let skip_buildings =
            fill.is_empty() && (inflight > 0 || !self.pending.is_empty() || !pin_settled);
        let b_vis = self.residency.buildings_visible();
        let chunks_pinned = rstats
            .get("chunks_pinned")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as u32;
        let trees = self.residency.world_tree_glyphs();
        let props = self.residency.world_prop_glyphs();
        let badges = self.residency.world_badge_glyphs();
        {
            let mut b = bridge.borrow_mut();
            b.tree_glyph_packed = self.residency.tree_glyph_count();
            b.heatmap_trees = self.residency.heatmap_trees_active();
        }
        {
            let mut g = engine.borrow_mut();
            let Some(e) = g.as_mut() else {
                return;
            };
            if !skip_buildings {
                e.upload_world_buildings(&fill, chunks_pinned, b_vis);
                e.upload_world_building_outlines(&outline, b_vis);
            }
            // Sticky empty mid-hydration for glyphs.
            if !trees.is_empty() || pin_settled {
                e.upload_icon_lane(0, &trees, true);
            }
            if !props.is_empty() || pin_settled {
                e.upload_icon_lane(1, &props, true);
            }
            if !badges.is_empty() || pin_settled {
                e.upload_icon_lane(2, &badges, true);
            }
            publish_engine(bridge, e);
        }
    }
}

impl Default for WorldHost {
    fn default() -> Self {
        Self::new()
    }
}

async fn load_glyph_atlas() -> Option<AtlasUpload> {
    let json_txt = fetch_text(ATLAS_JSON).await?;
    let json: serde_json::Value = serde_json::from_str(&json_txt).ok()?;
    let icons = json.get("icons")?.as_object()?;
    let mut keys: Vec<String> = icons.keys().cloned().collect();
    keys.sort();
    let webp = fetch_bytes(ATLAS_WEBP).await?;
    let (w, h, rgba) = decode_webp_rgba(&webp).await?;
    let mut uv = vec![0f32; keys.len() * 4];
    for (i, k) in keys.iter().enumerate() {
        let r = icons.get(k)?;
        let x = r.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let y = r.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let iw = r.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let ih = r.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let fw = w as f32;
        let fh = h as f32;
        uv[i * 4] = x / fw;
        uv[i * 4 + 1] = y / fh;
        uv[i * 4 + 2] = (x + iw) / fw;
        uv[i * 4 + 3] = (y + ih) / fh;
    }
    Some(AtlasUpload {
        rgba,
        w,
        h,
        uv,
        keys,
    })
}

async fn decode_webp_rgba(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let win = web_sys::window()?;
    let u8a = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
    u8a.copy_from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&u8a);
    let props = web_sys::BlobPropertyBag::new();
    props.set_type("image/webp");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &props).ok()?;
    let opts = web_sys::ImageBitmapOptions::new();
    opts.set_color_space_conversion(web_sys::ColorSpaceConversion::None);
    let p = win
        .create_image_bitmap_with_blob_and_image_bitmap_options(&blob, &opts)
        .ok()?;
    let bmp: web_sys::ImageBitmap = JsFuture::from(p).await.ok()?.dyn_into().ok()?;
    let w = bmp.width();
    let h = bmp.height();
    let canvas = web_sys::OffscreenCanvas::new(w, h).ok()?;
    let ctx = canvas
        .get_context("2d")
        .ok()
        .flatten()?
        .dyn_into::<web_sys::OffscreenCanvasRenderingContext2d>()
        .ok()?;
    ctx.draw_image_with_image_bitmap(&bmp, 0.0, 0.0).ok()?;
    bmp.close();
    let image_data = ctx
        .get_image_data(0.0, 0.0, f64::from(w), f64::from(h))
        .ok()?;
    Some((w, h, image_data.data().0))
}
