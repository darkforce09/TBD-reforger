//! T-166 W2–W5 — WorldStore + WorldResidency host (React `wgpuWorldLoader` port).

use std::collections::VecDeque;

use map_engine_core::geometry::polyline_strip::road_class_signature;
use map_engine_core::geometry::vector_compose::{
    compose_landcover_mesh, compose_roads_mesh, LandcoverInput, PolyMeshGpu, RoadInput, RoadMeshGpu,
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
/// T-173 H6 — airfield apron polygon lane (matches `draw_order::lane_role_from_u32` 8).
const ROLE_AIRFIELD_APRON: u32 = 8;
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
    /// T-173 P2 — cache of composed road meshes keyed by class-visibility signature (≤ 3 entries);
    /// replaces the recompose-per-0.5-zoom-band `last_road_band` string.
    road_meshes: std::collections::HashMap<u8, RoadMeshGpu>,
    last_road_sig: Option<u8>,
    /// T-173 P2 — landcover is a pure function of the 36 region hulls (no zoom input); compose +
    /// upload once, then only toggle visibility on the forest-fill band edge.
    landcover_mesh: Option<PolyMeshGpu>,
    landcover_shown: bool,
    /// T-173 P1/P2 — last `(buffers_revision, pin_settled, inflight_empty)` pushed to the engine;
    /// a settle pass whose tuple is unchanged skips the whole clone+upload block. `pin_settled` /
    /// `inflight_empty` ride along because the sticky mid-hydration building guard can flip on the
    /// last chunk's arrival without the revision advancing in that same pass.
    last_pushed: Option<(u64, bool, bool)>,
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
            road_meshes: std::collections::HashMap::new(),
            last_road_sig: None,
            landcover_mesh: None,
            landcover_shown: false,
            last_pushed: None,
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
                // T-173 H6 — derive the NW Everon airfield bbox from the runway segments so the
                // hangar/tower airfield glyphs (and the apron toggle) become live (T-152.5).
                let runways: Vec<_> = self.store.runway_segments().into_iter().cloned().collect();
                self.residency.set_airfield_bbox_from_runways(&runways);
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

    /// One residency settle pass. Returns whether it did real work (fetched, drained, or pushed a
    /// changed buffer) — `flush_viewport` breaks its multi-pass loop once every host reports idle
    /// (T-173 P2, kills the fixed ×6 recompose storm when nothing is pending).
    pub async fn run_viewport(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) -> bool {
        if !self.ready {
            return false;
        }
        self.ensure_atlas(engine, bridge);
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
        // T-173 P6/H7 — apply the operator's world-layer visibility prefs before composing. The
        // residency toggle setters + engine lane flips all early-out when unchanged, so this is a
        // cheap no-op on a settle where the prefs didn't move.
        self.apply_layer_prefs(engine);
        let roads_changed = self.push_roads(engine, zoom);
        let landcover_changed = self.push_landcover(engine);
        let missing = self
            .residency
            .set_viewport(bounds[0], bounds[1], bounds[2], bounds[3], zoom);
        // T-173 P3 — the old per-settle "invalidate every Some(0)/None draw id + re-set_viewport"
        // recovery loop is deleted: it refetched legit-empty chunks forever and cancelled in-flight
        // ones (double fetch). Empty/soft-fail handling now lives at the source — `ingest_chunk_gz`
        // marks well-formed-empty chunks `known_empty` (fetched once) and routes HTTP/parse
        // failures through `note_fetch_failure` (retry to a cap, then cache).
        let fetched = !missing.is_empty();
        if fetched {
            self.fetch_and_queue(missing).await;
        }
        let drained = self.drain(engine, bridge);
        let pushed = self.push_to_engine(engine, bridge);
        roads_changed || landcover_changed || fetched || drained || pushed
    }

    /// T-173 P6/H7 — push the per-user [`WorldLayerPrefs`] into the residency (glyph/fence/airfield
    /// recompose toggles) and the engine (standing vector/label-lane visibility). Idempotent: every
    /// setter early-returns when its value is unchanged.
    fn apply_layer_prefs(&mut self, engine: &EngineHandle) {
        let p = crate::world_layer_prefs::load_prefs();
        // Residency-driven (recompose) classes.
        self.residency
            .set_glyph_toggles(p.trees, p.props, p.buildings);
        self.residency.set_fences_toggle(p.fences);
        self.residency.set_airfield_toggle(p.airfield);
        // Standing-lane visibility flips (no re-upload).
        if let Some(e) = engine.borrow_mut().as_mut() {
            e.set_world_layer_visible("roads", p.roads);
            e.set_world_layer_visible("forest", p.forest);
            e.set_world_layer_visible("contours", p.contours);
            e.set_world_layer_visible("sea", p.sea);
            e.set_world_layer_visible("airfield", p.airfield);
            e.set_world_layer_visible("heights", p.heights);
            e.set_world_layer_visible("townLabels", p.town_labels);
            e.set_world_layer_visible("roadNames", p.road_names);
        }
    }

    /// T-173 H5 — the loaded road segments (for the road-name label placement host). Empty until
    /// `init` loads `roads.json.gz`.
    #[must_use]
    pub fn road_segments_clone(&self) -> Vec<map_engine_core::world::RoadSegment> {
        self.store.roads.clone()
    }

    /// T-173 H6 — build the airfield apron ground polygon from the DEM grid + runway-derived bbox
    /// and upload it to the `WorldAirfieldApron` lane (role 8). Static; built once at bootstrap.
    pub fn upload_airfield_apron(
        &self,
        engine: &EngineHandle,
        grid: &map_engine_core::dem::DemVectorGrid,
        visible: bool,
    ) {
        let Some(bbox) = self.residency.airfield_bbox() else {
            return;
        };
        let mesh = map_engine_core::world::build_airfield_apron_mesh(grid, bbox);
        if mesh.polygon_count == 0 {
            return;
        }
        if let Some(e) = engine.borrow_mut().as_mut() {
            e.upload_polygon_mesh(
                ROLE_AIRFIELD_APRON,
                &mesh.positions,
                &mesh.colors,
                &mesh.indices,
                mesh.polygon_count,
                visible,
            );
        }
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

    fn push_roads(&mut self, engine: &EngineHandle, zoom: f64) -> bool {
        if !self.roads_loaded {
            return false;
        }
        // T-173 P2 — the road mesh varies only by which class-visibility groups are on, not by the
        // continuous zoom: cache one packed mesh per signature (≤ 3) and re-upload the stored one
        // on a signature change instead of recomposing all 888 segments per 0.5-zoom band.
        let sig = road_class_signature(zoom);
        if self.last_road_sig == Some(sig) {
            return false;
        }
        self.last_road_sig = Some(sig);
        if !self.road_meshes.contains_key(&sig) {
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
            self.road_meshes
                .insert(sig, compose_roads_mesh(&inputs, zoom, true));
        }
        let mesh = &self.road_meshes[&sig];
        let vis = mesh.segment_count > 0;
        if let Some(e) = engine.borrow_mut().as_mut() {
            e.upload_strip_tris(ROLE_ROADS_CASING, &mesh.casing, mesh.segment_count, vis);
            e.upload_strip_tris(ROLE_ROADS, &mesh.centerline, mesh.segment_count, vis);
        }
        true
    }

    fn push_landcover(&mut self, engine: &EngineHandle) -> bool {
        if !self.landcover_ready {
            return false;
        }
        let vis = self.residency.forest_fill_effective();
        // T-173 P2 — compose the 36-hull mesh once (pure fn of the regions); afterward only toggle
        // lane visibility on the forest-fill band edge. No per-pass recompose/upload.
        if vis == self.landcover_shown && (self.landcover_mesh.is_some() || !vis) {
            return false;
        }
        if !vis {
            self.landcover_shown = false;
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.clear_vector_lane(ROLE_LANDCOVER);
            }
            return true;
        }
        if self.landcover_mesh.is_none() {
            let inputs: Vec<LandcoverInput<'_>> = self
                .store
                .regions
                .iter()
                .map(|r| LandcoverInput {
                    kind: r.kind.as_str(),
                    rings: r.polygon.as_slice(),
                })
                .collect();
            self.landcover_mesh = Some(compose_landcover_mesh(&inputs));
        }
        self.landcover_shown = true;
        if let Some(mesh) = &self.landcover_mesh {
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
        true
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

    fn drain(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) -> bool {
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
                // T-173 P3 — disposition owned by core: Applied/ParsedEmpty keep the result;
                // ShapeMismatch + gzip/json errors route through the retry-capped failure path.
                Some(bytes) => match self.residency.ingest_chunk_gz(&next.id, &bytes) {
                    Ok(_) => {}
                    Err(_) => self.residency.note_fetch_failure(&next.id),
                },
                None => self.residency.note_fetch_failure(&next.id),
            }
            applied += 1;
        }
        if applied > 0 {
            self.residency.end_ingest_frame_at(js_sys::Date::now());
            self.push_to_engine(engine, bridge);
        }
        applied > 0
    }

    /// Push the residency's GPU-facing buffers to the engine. Returns whether any upload ran.
    ///
    /// T-173 P1/P2 — revision gate: the residency bumps `buffers_revision` only on a real
    /// recompose, so a settle pass whose `(revision, pin_settled, inflight_empty)` tuple is
    /// unchanged skips the whole clone+upload block (the ×6 flush + drain re-push storm). The
    /// bridge stats still refresh cheaply so the HUD/probe stay live.
    fn push_to_engine(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) -> bool {
        let revision = self.residency.buffers_revision();
        let pin_settled = self.residency.pin_settled();
        let inflight_empty = self.residency.inflight_count() == 0;
        let gate = (revision, pin_settled, inflight_empty);
        if self.last_pushed == Some(gate) {
            // Nothing recomposed since the last push — refresh the cheap bridge mirror and bail.
            let mut b = bridge.borrow_mut();
            b.merge_residency_stats(&self.residency.stats_json());
            b.tree_glyph_packed = self.residency.tree_glyph_count();
            return false;
        }
        self.last_pushed = Some(gate);

        let fill = self.residency.world_building_fill();
        let outline = self.residency.world_building_outline();
        let stats = self.residency.stats_json();
        // Sticky empty mid-hydration for *buildings* only — never skip glyph lanes when the
        // viewport is tree-only (zoom-in probe at forest center has fill=[] while trees pack).
        let skip_buildings =
            fill.is_empty() && (!inflight_empty || !self.pending.is_empty() || !pin_settled);
        let b_vis = self.residency.buildings_visible();
        let chunks_pinned = self.residency.chunks_resident() as u32;
        let trees = self.residency.world_tree_glyphs();
        let props = self.residency.world_prop_glyphs();
        let badges = self.residency.world_badge_glyphs();
        // T-173 P9 — bridge the fence/pier/rail strip lane the React host (T-152.15) drove and the
        // Leptos port dropped. Gated by `strips_visible()` (fence z ≥ 1.5, pier z ≥ −1.0, decoupled
        // toggles); the packed buffer is anchor-rewritten + drawn by the engine's WorldFences lane.
        let strips = self.residency.world_fence_strips();
        let strip_vis = self.residency.strips_visible();
        let strip_count = self.residency.fence_strip_segment_count()
            + self.residency.pier_strip_segment_count()
            + self.residency.bridge_rail_strip_count();
        {
            let mut b = bridge.borrow_mut();
            b.tree_glyph_packed = self.residency.tree_glyph_count();
            b.merge_residency_stats(&stats);
        }
        {
            let mut g = engine.borrow_mut();
            let Some(e) = g.as_mut() else {
                return false;
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
            e.upload_world_fence_strips(&strips, strip_count, strip_vis);
            publish_engine(bridge, e);
        }
        true
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
