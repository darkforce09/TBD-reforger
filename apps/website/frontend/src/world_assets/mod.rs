//! T-166 — Leptos map-asset host: hillshade + unified satellite + DEM vectors + world residency
//! + forest mass. Engine pipelines live in `map-engine-*`; this module is fetch → decode → upload.

#![cfg(target_arch = "wasm32")]

mod bridge;
mod dem_vectors;
mod fetch;
mod forest_mass;
mod labels;
mod satellite;
mod tbd_sat;
mod world_host;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use map_engine_core::dem::hillshade::build_hillshade_image;
use map_engine_core::dem::png_decode::decode_png_to_meters;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::select_tool::EngineHandle;

use bridge::{new_bridge, publish, publish_engine, BridgeHandle};
use dem_vectors::DemVectors;
use fetch::fetch_bytes;
use forest_mass::ForestMassHost;
use world_host::WorldHost;

const TERRAIN_M: f64 = 12_800.0;

/// Shared host handle for camera-settle refresh.
pub type HostHandle = Rc<RefCell<Option<MapHost>>>;

/// Shared handle to the retained DEM vector grid — published once by `bootstrap`, read by the
/// editor's pointer-move for the CUR Z readout (T-172 B2). Separate from `HostHandle` because
/// `flush_viewport` takes the host out during async passes (the grid must stay reachable).
pub type DemGridHandle = Rc<RefCell<Option<Rc<map_engine_core::dem::DemVectorGrid>>>>;

pub fn new_dem_grid_handle() -> DemGridHandle {
    Rc::new(RefCell::new(None))
}

pub struct MapHost {
    bridge: BridgeHandle,
    world: WorldHost,
    forest: ForestMassHost,
    dem: DemVectors,
    settle_timer: Rc<Cell<Option<i32>>>,
    /// T-173 P1 — deadline (ms, `Date.now` clock) by which the next settle must fire even if pan
    /// input keeps coming. `0.0` means "no settle pending"; set on the first schedule after idle.
    settle_deadline: Rc<Cell<f64>>,
    /// T-173 P6/H8 — terrain id (for the Map basemap pyramid url).
    terrain: String,
    /// T-173 H5 — town / road / height text-label host.
    labels: labels::LabelHost,
}

impl MapHost {
    fn new() -> Self {
        Self {
            bridge: new_bridge(),
            world: WorldHost::new(),
            forest: ForestMassHost::new(),
            dem: DemVectors::new(),
            settle_timer: Rc::new(Cell::new(None)),
            settle_deadline: Rc::new(Cell::new(0.0)),
            terrain: String::new(),
            labels: labels::LabelHost::new(),
        }
    }

    /// T-173 P6/H8 — swap the basemap texture between the unified Satellite lane and the stylized
    /// Map cartographic pyramid. Falls back to satellite when the Map pyramid is absent (tiles not
    /// built locally via `make map-cartographic-everon`). `&self` (only reads `terrain`) so the
    /// caller never holds the host `RefCell` borrow across the load `.await`.
    async fn set_basemap_view(&self, engine: &EngineHandle, view: &str) {
        swap_basemap(engine, &self.terrain, view).await;
    }
}

/// T-173 P6/H8 — the basemap swap itself, decoupled from the host borrow (so the async load never
/// runs while `host.borrow_mut()` is held — that footgun freezes camera input, per CLAUDE.md).
async fn swap_basemap(engine: &EngineHandle, terrain: &str, view: &str) {
    if view == "map" {
        let ok = satellite::load_map_basemap(engine, terrain, TERRAIN_M, TERRAIN_M).await;
        if !ok {
            leptos::logging::warn!("map basemap tiles unavailable — falling back to satellite");
            satellite::show_satellite_basemap(engine);
        }
    } else {
        satellite::show_satellite_basemap(engine);
    }
}

pub fn new_host_handle() -> HostHandle {
    Rc::new(RefCell::new(None))
}

thread_local! {
    /// T-173 P6 — engine + host handles for the Mission Settings render-pref controls. Registered
    /// once the editor's engine + map host exist; the dialog's `on:change` handlers reach the live
    /// map through these without threading handles through the chrome component tree.
    static RENDER_CTX: RefCell<Option<(EngineHandle, HostHandle)>> = const { RefCell::new(None) };
}

/// Register the engine + host so the Mission Settings dialog can apply render prefs (P6).
pub fn register_render_ctx(engine: EngineHandle, host: HostHandle) {
    RENDER_CTX.with(|c| *c.borrow_mut() = Some((engine, host)));
}

thread_local! {
    /// T-176 B2 — true while a pan gesture is active (pointer down → up). During a pan,
    /// `flush_viewport` skips the heavy zoom-band marching-squares rebuilds (DEM contour/sea +
    /// the 8 m forest mass) so a simultaneous wheel-zoom doesn't rebuild them on every ~250 ms
    /// forced settle mid-drag (the zoom+pan stutter). World chunk residency still streams. The
    /// gesture-end (pointer-up) settle runs the full recompute once. A thread-local (not a MapHost
    /// field) so it stays readable while `flush_viewport` has taken the host out.
    static CAMERA_GESTURE: Cell<bool> = const { Cell::new(false) };
}

/// T-176 B2 — mark a camera pan gesture active/inactive (mission_editor pointer down/up on MMB/RMB).
pub fn set_camera_gesture(active: bool) {
    CAMERA_GESTURE.with(|g| g.set(active));
}

fn camera_gesture_active() -> bool {
    CAMERA_GESTURE.with(Cell::get)
}

/// T-173 P6 — hillshade overlay on/off + strength (React `useDemLayer` role-1 lane, live re-tint,
/// no texture rebuild). `opacity` is 0..1.
pub fn apply_hillshade(visible: bool, opacity: f64) {
    RENDER_CTX.with(|c| {
        if let Some((engine, _)) = c.borrow().as_ref() {
            if let Some(e) = engine.borrow_mut().as_mut() {
                #[allow(clippy::cast_possible_truncation)]
                e.set_lane_opacity(1, opacity as f32, visible);
            }
        }
    });
}

/// T-173 P6 — procedural grid on/off over the basemap.
pub fn apply_grid(visible: bool) {
    RENDER_CTX.with(|c| {
        if let Some((engine, _)) = c.borrow().as_ref() {
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.set_grid(TERRAIN_M, TERRAIN_M, true, visible);
            }
        }
    });
}

/// T-173 P6 — re-apply the world-layer prefs (schedule a settle; the host reads them each pass).
pub fn refresh_world_layers() {
    RENDER_CTX.with(|c| {
        if let Some((engine, host)) = c.borrow().as_ref() {
            schedule_camera_settle(host.clone(), engine.clone());
        }
    });
}

/// T-173 P6/H8 — swap the basemap between the unified satellite texture and the cartographic Map
/// pyramid. `view` is `"satellite"` or `"map"`.
pub fn apply_basemap_view(view: &str) {
    RENDER_CTX.with(|c| {
        if let Some((engine, host)) = c.borrow().as_ref() {
            // Read the terrain id under a short borrow, then drop it — the async tile load must not
            // run while the host `RefCell` is borrowed (CLAUDE.md: that freezes camera input).
            let terrain = host
                .borrow()
                .as_ref()
                .map(|mh| mh.terrain.clone())
                .unwrap_or_default();
            let engine = engine.clone();
            let view = view.to_string();
            wasm_bindgen_futures::spawn_local(async move {
                swap_basemap(&engine, &terrain, &view).await;
            });
        }
    });
}

/// Mount-time bootstrap: hillshade + sat + DEM vectors + world + forest, then first settle.
pub async fn bootstrap(
    engine: EngineHandle,
    terrain: String,
    host: HostHandle,
    dem_out: DemGridHandle,
) {
    let mut mh = MapHost::new();
    mh.terrain = terrain.clone();
    let bridge = mh.bridge.clone();
    let base = format!("/map-assets/{terrain}");

    // One manifest fetch feeds both the DEM/hillshade and satellite paths (T-172 H4 — this was
    // fetched twice), and the two loads run concurrently: the multi-MB satellite fetch overlaps
    // the DEM decode + hillshade CPU work instead of waiting behind it (T-172 B1).
    let manifest: Option<ManifestDem> = match fetch_bytes(&format!("{base}/manifest.json")).await {
        Some(bytes) => serde_json::from_slice(&bytes).ok(),
        None => None,
    };
    let dem_fut = async { load_dem_and_hillshade(&engine, &base, manifest.as_ref()?).await };
    let sat_fut = async {
        let (url, tw, th) = sat_url_from(manifest.as_ref()?, &base)?;
        satellite::load_satellite(engine.clone(), &base, &url, tw, th, bridge.clone()).await;
        Some(())
    };
    let (dem_res, _sat) = futures::join!(dem_fut, sat_fut);

    // T-173 H5 — keep the decoded DEM raster so the label host can find peaks after the world's
    // roads load (peaks + road/town labels all init together below).
    let mut dem_kept: Option<(Vec<f32>, u32, u32)> = None;
    if let Some((meters, w, h, hs_w, hs_h)) = dem_res {
        {
            let mut b = bridge.borrow_mut();
            b.hillshade_w = hs_w;
            b.hillshade_h = hs_h;
        }
        publish(&bridge);
        mh.dem.ensure_grid(&meters, w, h);
        *dem_out.borrow_mut() = mh.dem.grid();
        let zoom = engine.borrow().as_ref().map(|e| e.zoom()).unwrap_or(-2.0);
        mh.dem.sync(&engine, zoom);
        dem_kept = Some((meters, w, h));
    }

    if let Some(e) = engine.borrow_mut().as_mut() {
        e.set_grid(TERRAIN_M, TERRAIN_M, true, true);
    }

    // T-173 P6 — restore the saved render prefs on load: hillshade on/off + strength and grid come
    // from the mission's `meta.environment`; the basemap view is a per-user localStorage pref. The
    // world-layer toggles are applied by the world host each settle.
    {
        let env = crate::editor_ops::read_env();
        if let Some(e) = engine.borrow_mut().as_mut() {
            #[allow(clippy::cast_possible_truncation)]
            e.set_lane_opacity(1, env.hillshade_opacity as f32, env.show_hillshade);
            e.set_grid(TERRAIN_M, TERRAIN_M, true, env.show_grid);
        }
        let view = crate::world_layer_prefs::load_basemap_view();
        if view == "map" {
            mh.set_basemap_view(&engine, &view).await;
        }
    }

    let _ = mh.world.init(&terrain).await;
    mh.forest.init(&terrain);

    // T-173 H6 — build + upload the airfield apron ground polygon once (static). Needs both the
    // runway-derived bbox (set in `world.init`) and the DEM grid.
    if let Some(grid) = mh.dem.grid() {
        let show = crate::world_layer_prefs::load_prefs().airfield;
        mh.world.upload_airfield_apron(&engine, &grid, show);
    }

    // T-173 H5 — init the text-label host (town/road/height) now that both the DEM raster and the
    // road segments are available, then push once for the initial camera.
    if let Some((meters, w, h)) = dem_kept {
        let roads = mh.world.road_segments_clone();
        mh.labels.init(&base, &meters, w, h, roads).await;
        let zoom = engine.borrow().as_ref().map(|e| e.zoom()).unwrap_or(-2.0);
        let prefs = crate::world_layer_prefs::load_prefs();
        mh.labels.push(&engine, zoom, &prefs);
    }

    // Drain residency / forest over a few passes so the first paint settles without requiring a pan.
    // Each pass awaits chunk/density fetches, so the browser event loop advances between iterations.
    // T-173 P2 — break as soon as both hosts report idle instead of always running 12 passes.
    for _ in 0..12 {
        let w = mh.world.run_viewport(&engine, &bridge).await;
        let f = mh.forest.run_viewport(&engine, &bridge).await;
        if !w && !f {
            break;
        }
    }

    if let Some(e) = engine.borrow().as_ref() {
        publish_engine(&bridge, e);
    } else {
        publish(&bridge);
    }

    *host.borrow_mut() = Some(mh);
}

/// Immediate viewport refresh (smoke probe / tests). Also used by the debounced settle path.
/// Runs several passes so a soft-fail chunk re-fetch + ingest budget can settle.
///
/// **Must not hold `host.borrow_mut()` across `.await`** — pan/wheel settle and the rAF loop
/// also touch the host; a RefCell panic freezes camera input for the rest of the session.
pub fn flush_viewport(host: HostHandle, engine: EngineHandle) {
    wasm_bindgen_futures::spawn_local(async move {
        for _ in 0..6 {
            // Take the host out for the async pass so JS event handlers can still
            // `host.borrow()` (they no-op while `None`) without panicking.
            let mut h = {
                let mut g = host.borrow_mut();
                match g.take() {
                    Some(h) => h,
                    None => return,
                }
            };
            let zoom = engine.borrow().as_ref().map(|e| e.zoom()).unwrap_or(-2.0);
            // T-176 B2 — while a pan gesture is active, defer the heavy zoom-band marching-squares
            // rebuilds (DEM contour/sea via `dem.sync`, and the 8 m forest mass) — a simultaneous
            // wheel-zoom changes their band, and rebuilding on every ~250 ms forced settle mid-drag
            // is the zoom+pan stutter (the 8 m forest is ~16× the old compose). World chunk
            // residency below still streams so buildings/roads fill as you pan; the gesture-end
            // (pointer-up) settle re-runs this with the gesture flag clear → one full recompute.
            let gesture = camera_gesture_active();
            if !gesture {
                h.dem.sync(&engine, zoom);
            }
            // T-173 P2 — the memoized hosts now report whether a pass changed anything; once both
            // are idle the remaining passes would be pure no-ops (revision-gated), so stop early.
            // Multi-pass hydration still works: pass N ingests what pass N-1 fetched, and each of
            // those passes reports `did_work=true` until the viewport is fully resident.
            let did_world = h.world.run_viewport(&engine, &h.bridge).await;
            // T-178 — after the island tex is up, LOD params are cheap: run during gesture so
            // outline/α update mid-pan. Never start the 625-bin boot mid-gesture (blocks input).
            let did_forest = if gesture && !h.forest.is_uploaded() {
                false
            } else {
                h.forest.run_viewport(&engine, &h.bridge).await
            };
            // T-173 H5 — repack the text labels for the current zoom band (memoized; cheap no-op
            // when the band + toggles are unchanged).
            {
                let prefs = crate::world_layer_prefs::load_prefs();
                h.labels.push(&engine, zoom, &prefs);
            }
            if let Some(e) = engine.borrow().as_ref() {
                publish_engine(&h.bridge, e);
            }
            *host.borrow_mut() = Some(h);
            if !did_world && !did_forest {
                break;
            }
        }
    });
}

/// Residency / forest / DEM-vector refresh after camera moves — debounce with a max-latency arm
/// (T-173 P1). A quiet gap of 120 ms fires a settle (the old behaviour); but under continuous pan
/// where the debounce would never expire, the deadline forces a settle every ~250 ms so chunks
/// stream in mid-drag (≈4×/s) instead of freezing until pointer-up. Each streamed settle is cheap
/// because the residency memo skips recompose when no chunk boundary was crossed.
const SETTLE_DEBOUNCE_MS: f64 = 120.0;
const SETTLE_MAX_LATENCY_MS: f64 = 250.0;

pub fn schedule_camera_settle(host: HostHandle, engine: EngineHandle) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let (timer_slot, deadline_slot) = {
        let g = host.borrow();
        let Some(h) = g.as_ref() else {
            return;
        };
        (h.settle_timer.clone(), h.settle_deadline.clone())
    };
    let now = js_sys::Date::now();
    // First schedule after idle sets the hard deadline; subsequent re-schedules keep it.
    if deadline_slot.get() <= 0.0 {
        deadline_slot.set(now + SETTLE_MAX_LATENCY_MS);
    }
    let delay = (deadline_slot.get() - now).clamp(0.0, SETTLE_DEBOUNCE_MS);
    if let Some(id) = timer_slot.get() {
        win.clear_timeout_with_handle(id);
    }
    let host2 = host.clone();
    let eng2 = engine.clone();
    let slot2 = timer_slot.clone();
    let deadline2 = deadline_slot.clone();
    let cb = Closure::once_into_js(move || {
        slot2.set(None);
        deadline2.set(0.0);
        flush_viewport(host2, eng2);
    });
    #[allow(clippy::cast_possible_truncation)]
    if let Ok(id) = win.set_timeout_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        delay as i32,
    ) {
        timer_slot.set(Some(id));
    }
}

#[derive(serde::Deserialize)]
struct ManifestDem {
    #[serde(rename = "worldBounds")]
    world_bounds: [f64; 4],
    dem: DemInfo,
    tiles: Option<TilesBlock>,
}
#[derive(serde::Deserialize)]
struct DemInfo {
    path: String,
    #[serde(rename = "heightRangeMinM")]
    min_m: f64,
    #[serde(rename = "heightRangeMaxM")]
    max_m: f64,
}
#[derive(serde::Deserialize)]
struct TilesBlock {
    satellite: Option<SatBlock>,
}
#[derive(serde::Deserialize)]
struct SatBlock {
    unified: Option<UnifiedBlock>,
}
#[derive(serde::Deserialize)]
struct UnifiedBlock {
    url: Option<String>,
    path: Option<String>,
}

async fn load_dem_and_hillshade(
    engine: &EngineHandle,
    base: &str,
    manifest: &ManifestDem,
) -> Option<(Vec<f32>, u32, u32, u32, u32)> {
    let dem_bytes = fetch_bytes(&format!("{base}/{}", manifest.dem.path)).await?;
    let dem = decode_png_to_meters(&dem_bytes, manifest.dem.min_m, manifest.dem.max_m).ok()?;
    let hs = build_hillshade_image(&dem.meters, dem.width as usize, dem.height as usize);
    if hs.data.is_empty() || hs.w == 0 || hs.h == 0 {
        return None;
    }
    let [min_x, min_y, max_x, max_y] = manifest.world_bounds;
    let (w, h) = (hs.w as u32, hs.h as u32);
    {
        let mut guard = engine.borrow_mut();
        let e = guard.as_mut()?;
        e.tex_layer_begin(1, min_x, min_y, max_x, max_y, w, h, 1, 3)
            .ok()?;
        e.tex_layer_write_rgba(1, 0, 0, 0, w, h, &hs.data).ok()?;
        e.tex_layer_commit(1, 0.4, true).ok()?;
    }
    Some((dem.meters, dem.width, dem.height, w, h))
}

fn sat_url_from(manifest: &ManifestDem, base: &str) -> Option<(String, f64, f64)> {
    let u = manifest
        .tiles
        .as_ref()?
        .satellite
        .as_ref()?
        .unified
        .as_ref()?;
    let url = u
        .url
        .clone()
        .or_else(|| u.path.as_ref().map(|p| format!("{base}/{p}")))?;
    let [_, _, max_x, max_y] = manifest.world_bounds;
    Some((url, max_x, max_y))
}
