//! T-166 — Leptos map-asset host: hillshade + unified satellite + DEM vectors + world residency
//! + forest mass. Engine pipelines live in `map-engine-*`; this module is fetch → decode → upload.

#![cfg(target_arch = "wasm32")]

mod bridge;
mod dem_vectors;
mod fetch;
mod forest_mass;
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

pub struct MapHost {
    bridge: BridgeHandle,
    world: WorldHost,
    forest: ForestMassHost,
    dem: DemVectors,
    settle_timer: Rc<Cell<Option<i32>>>,
}

impl MapHost {
    fn new() -> Self {
        Self {
            bridge: new_bridge(),
            world: WorldHost::new(),
            forest: ForestMassHost::new(),
            dem: DemVectors::new(),
            settle_timer: Rc::new(Cell::new(None)),
        }
    }
}

pub fn new_host_handle() -> HostHandle {
    Rc::new(RefCell::new(None))
}

/// Mount-time bootstrap: hillshade + sat + DEM vectors + world + forest, then first settle.
pub async fn bootstrap(engine: EngineHandle, terrain: String, host: HostHandle) {
    let mut mh = MapHost::new();
    let bridge = mh.bridge.clone();

    if let Some((meters, w, h, hs_w, hs_h)) = load_dem_and_hillshade(&engine, &terrain).await {
        {
            let mut b = bridge.borrow_mut();
            b.hillshade_w = hs_w;
            b.hillshade_h = hs_h;
        }
        publish(&bridge);
        mh.dem.ensure_grid(&meters, w, h);
        let zoom = engine.borrow().as_ref().map(|e| e.zoom()).unwrap_or(-2.0);
        mh.dem.sync(&engine, zoom);
    }

    if let Some(e) = engine.borrow_mut().as_mut() {
        e.set_grid(TERRAIN_M, TERRAIN_M, true, true);
    }

    if let Some((url, tw, th)) = sat_url_for_terrain(&terrain).await {
        satellite::load_satellite(
            engine.clone(),
            &format!("/map-assets/{terrain}"),
            &url,
            tw,
            th,
            bridge.clone(),
        )
        .await;
    }

    let _ = mh.world.init(&terrain).await;
    mh.forest.init(&terrain);

    // Drain residency / forest over a few passes so the first paint settles without requiring a pan.
    // Each pass awaits chunk/density fetches, so the browser event loop advances between iterations.
    for _ in 0..12 {
        mh.world.run_viewport(&engine, &bridge).await;
        mh.forest.run_viewport(&engine, &bridge).await;
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
            h.dem.sync(&engine, zoom);
            h.world.run_viewport(&engine, &h.bridge).await;
            h.forest.run_viewport(&engine, &h.bridge).await;
            if let Some(e) = engine.borrow().as_ref() {
                publish_engine(&h.bridge, e);
            }
            *host.borrow_mut() = Some(h);
        }
    });
}

/// Debounced (120 ms) residency / forest / DEM-vector refresh after camera moves.
pub fn schedule_camera_settle(host: HostHandle, engine: EngineHandle) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let timer_slot = {
        let g = host.borrow();
        let Some(h) = g.as_ref() else {
            return;
        };
        h.settle_timer.clone()
    };
    if let Some(id) = timer_slot.get() {
        win.clear_timeout_with_handle(id);
    }
    let host2 = host.clone();
    let eng2 = engine.clone();
    let slot2 = timer_slot.clone();
    let cb = Closure::once_into_js(move || {
        slot2.set(None);
        flush_viewport(host2, eng2);
    });
    if let Ok(id) =
        win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 120)
    {
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
    terrain: &str,
) -> Option<(Vec<f32>, u32, u32, u32, u32)> {
    let base = format!("/map-assets/{terrain}");
    let bytes = fetch_bytes(&format!("{base}/manifest.json")).await?;
    let manifest: ManifestDem = serde_json::from_slice(&bytes).ok()?;
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

async fn sat_url_for_terrain(terrain: &str) -> Option<(String, f64, f64)> {
    let base = format!("/map-assets/{terrain}");
    let bytes = fetch_bytes(&format!("{base}/manifest.json")).await?;
    let manifest: ManifestDem = serde_json::from_slice(&bytes).ok()?;
    let u = manifest.tiles?.satellite?.unified?;
    let url = u.url.or_else(|| u.path.map(|p| format!("{base}/{p}")))?;
    let [_, _, max_x, max_y] = manifest.world_bounds;
    Some((url, max_x, max_y))
}
