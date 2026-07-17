//! T-159.28 — the map-asset host (MVP: terrain hillshade). Before this the Leptos editor rendered
//! only a bare grid; now it fetches the Everon DEM and paints a hillshade relief lane under the
//! grid, so the editor shows terrain.
//!
//! The heavy lifting is ALREADY Rust in `map-engine-core::dem` (the React TS glue only fetched bytes
//! and called into the engine): `decode_png_to_meters` (the 16-bit DEM PNG → `Vec<f32>` meters) and
//! `build_hillshade_image` (Horn hillshade → downsampled RGBA). This host is the thin fetch +
//! upload layer: manifest → DEM bytes → decode → hillshade → `tex_layer_*` (role 1 = hillshade,
//! mode 3), the same engine API the React `useDemLayer` fed.
//!
//! **Deferred (folded forward):** the unified satellite basemap (`everon-sat.tbd-sat`, 152 MB, its
//! own mip-chain format) and world-object streaming (315 chunks / 1.2 M instances — the
//! `map-engine-core::world` parser + residency). The hillshade MVP is the P0 "editor shows terrain"
//! value; the rest rides a T-159.28 follow-on.
#![cfg(target_arch = "wasm32")]

use map_engine_core::dem::hillshade::build_hillshade_image;
use map_engine_core::dem::png_decode::decode_png_to_meters;

use crate::select_tool::EngineHandle;

/// The terrain manifest fields the hillshade path needs (`/map-assets/<terrain>/manifest.json`).
#[derive(serde::Deserialize)]
struct Manifest {
    #[serde(rename = "worldBounds")]
    world_bounds: [f64; 4],
    dem: DemInfo,
}
#[derive(serde::Deserialize)]
struct DemInfo {
    path: String,
    #[serde(rename = "heightRangeMinM")]
    min_m: f64,
    #[serde(rename = "heightRangeMaxM")]
    max_m: f64,
}

/// Fetch bytes from a same-origin URL (the Trunk `/map-assets` proxy in dev; the backend ServeDir in
/// prod). Returns `None` on any error — the editor stays on the bare grid, never blocks.
async fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    let resp = gloo_net::http::Request::get(url).send().await.ok()?;
    if !(200..300).contains(&resp.status()) {
        return None;
    }
    resp.binary().await.ok()
}

/// Load the terrain hillshade for `terrain` and upload it as the engine's hillshade lane (role 1).
/// Runs in `spawn_local` from the editor mount, after the engine is `Some`. All work is off the
/// render path; the borrow of the engine is scoped to the synchronous upload block (no borrow spans
/// an `.await`, matching the persist swap discipline).
pub async fn load_hillshade(engine: EngineHandle, terrain: String) {
    let base = format!("/map-assets/{terrain}");
    let Some(manifest_bytes) = fetch_bytes(&format!("{base}/manifest.json")).await else {
        return;
    };
    let Ok(manifest) = serde_json::from_slice::<Manifest>(&manifest_bytes) else {
        return;
    };
    let Some(dem_bytes) = fetch_bytes(&format!("{base}/{}", manifest.dem.path)).await else {
        return;
    };
    // Decode the 16-bit DEM → meters, then Horn hillshade (self-downsampled to MAX_EDGE). Both are
    // the existing Rust core — no JS decode, no ImageBitmap.
    let Ok(dem) = decode_png_to_meters(&dem_bytes, manifest.dem.min_m, manifest.dem.max_m) else {
        return;
    };
    let hs = build_hillshade_image(&dem.meters, dem.width as usize, dem.height as usize);
    if hs.data.is_empty() || hs.w == 0 || hs.h == 0 {
        return;
    }

    let [min_x, min_y, max_x, max_y] = manifest.world_bounds;
    let (w, h) = (hs.w as u32, hs.h as u32);
    let mut guard = engine.borrow_mut();
    let Some(e) = guard.as_mut() else {
        return;
    };
    // role 1 = hillshade, mode 3 = hillshade (see engine `BasemapRenderMode`). One mip, one block.
    if e.tex_layer_begin(1, min_x, min_y, max_x, max_y, w, h, 1, 3)
        .is_err()
    {
        return;
    }
    if e.tex_layer_write_rgba(1, 0, 0, 0, w, h, &hs.data).is_err() {
        return;
    }
    // Commit visible at the React default hillshade opacity (0.4 blend under the grid + slots).
    let _ = e.tex_layer_commit(1, 0.4, true);
    // Install a smoke bridge so the GPU gate can prove the lane exists (dims + a stats read).
    install_bridge(w, h);
}

/// `window.__mapAssets` — the read-only GPU-gate bridge: the uploaded hillshade dims (proven the
/// decode+upload ran) so the headless driver can assert terrain is present without a screenshot.
fn install_bridge(w: u32, h: u32) {
    use wasm_bindgen::JsValue;
    let Some(win) = web_sys::window() else {
        return;
    };
    let obj = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("hillshadeW"),
        &JsValue::from_f64(f64::from(w)),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("hillshadeH"),
        &JsValue::from_f64(f64::from(h)),
    );
    let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__mapAssets"), &obj);
}
