//! T-166 — unified satellite host: TBDS Range preview + optional full mip upload.
//! CI / `?sat=preview` never GETs the 146–206 MB bundle body (Range only).

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::select_tool::EngineHandle;

use super::bridge::{publish, BridgeHandle};
use super::fetch::{fetch_bytes, fetch_range};
use super::tbd_sat::{
    parse_tbd_sat, parse_tbd_sat_index_only, pick_base_level, pick_preview_level, TbdSatIndex,
    TbdSatMip,
};

const ROLE_BASEMAP: u32 = 0;
const MODE_UNIFIED: u32 = 0;
const MODE_SINGLE: u32 = 2;
const PREVIEW_MAX_EDGE: u32 = 1024;

/// `?sat=preview` — Range-only path; never full-bundle GET (CI / gate harness).
pub fn sat_preview_only() -> bool {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .map(|s| s.contains("sat=preview"))
        .unwrap_or(false)
}

enum Decoded {
    Bitmap(web_sys::ImageBitmap),
    Rgba { w: u32, h: u32, rgba: Vec<u8> },
}

async fn decode_webp(bytes: &[u8], webgl2: bool) -> Option<Decoded> {
    let win = web_sys::window()?;
    let u8 = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
    u8.copy_from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&u8);
    let props = web_sys::BlobPropertyBag::new();
    props.set_type("image/webp");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &props).ok()?;
    let opts = web_sys::ImageBitmapOptions::new();
    opts.set_color_space_conversion(web_sys::ColorSpaceConversion::None);
    let p = win
        .create_image_bitmap_with_blob_and_image_bitmap_options(&blob, &opts)
        .ok()?;
    let bmp: web_sys::ImageBitmap = JsFuture::from(p).await.ok()?.dyn_into().ok()?;
    if !webgl2 {
        return Some(Decoded::Bitmap(bmp));
    }
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
    let data = image_data.data().0;
    Some(Decoded::Rgba { w, h, rgba: data })
}

fn upload_decoded(
    engine: &mut map_engine_render::RenderEngine,
    role: u32,
    mip: u32,
    x: u32,
    y: u32,
    decoded: Decoded,
) -> bool {
    match decoded {
        Decoded::Bitmap(bmp) => {
            let w = bmp.width();
            let h = bmp.height();
            engine
                .tex_layer_write_bitmap(role, mip, x, y, w, h, bmp)
                .is_ok()
        }
        Decoded::Rgba { w, h, rgba } => engine
            .tex_layer_write_rgba(role, mip, x, y, w, h, &rgba)
            .is_ok(),
    }
}

async fn fetch_index_head(url: &str) -> Option<(TbdSatIndex, u64)> {
    let head = fetch_range(url, 0, 11).await?;
    if head.bytes.len() < 12 {
        return None;
    }
    let version = u32::from_le_bytes(head.bytes[4..8].try_into().ok()?);
    let json_len = u32::from_le_bytes(head.bytes[8..12].try_into().ok()?);
    if version != 1 || json_len == 0 || json_len > 16 * 1024 * 1024 {
        return None;
    }
    let full = fetch_range(url, 0, 11 + u64::from(json_len)).await?;
    let index = parse_tbd_sat_index_only(&full.bytes, full.total).ok()?;
    Some((index, full.total))
}

async fn fetch_mip_blocks(
    url: &str,
    mip: &TbdSatMip,
) -> Option<Vec<(super::tbd_sat::TbdSatTile, Vec<u8>)>> {
    let mut out = Vec::with_capacity(mip.tiles.len());
    for tile in &mip.tiles {
        let end = tile.offset + tile.length - 1;
        let r = fetch_range(url, tile.offset, end).await?;
        if r.bytes.len() as u64 != tile.length {
            return None;
        }
        out.push((tile.clone(), r.bytes));
    }
    Some(out)
}

async fn commit_mip(
    engine: &EngineHandle,
    terrain_w: f64,
    terrain_h: f64,
    mip: &TbdSatMip,
    blocks: Vec<(super::tbd_sat::TbdSatTile, Vec<u8>)>,
    mode: u32,
    mip_count: u32,
    opacity: f64,
) -> bool {
    let webgl2 = {
        let g = engine.borrow();
        g.as_ref().map(|e| e.backend() == "webgl2").unwrap_or(true)
    };
    let mut decoded = Vec::with_capacity(blocks.len());
    for (tile, bytes) in &blocks {
        let Some(d) = decode_webp(bytes, webgl2).await else {
            return false;
        };
        decoded.push((tile.clone(), d));
    }
    let mut guard = engine.borrow_mut();
    let Some(e) = guard.as_mut() else {
        return false;
    };
    if e.tex_layer_begin(
        ROLE_BASEMAP,
        0.0,
        0.0,
        terrain_w,
        terrain_h,
        mip.width,
        mip.height,
        mip_count,
        mode,
    )
    .is_err()
    {
        return false;
    }
    for (tile, d) in decoded {
        if !upload_decoded(e, ROLE_BASEMAP, 0, tile.x, tile.y, d) {
            return false;
        }
    }
    e.tex_layer_commit(ROLE_BASEMAP, opacity as f32, true)
        .is_ok()
}

/// Range-preview one coarse mip (≤1024 px), mode=single. Best-effort.
async fn try_preview(
    engine: &EngineHandle,
    url: &str,
    terrain_w: f64,
    terrain_h: f64,
    bridge: &BridgeHandle,
) -> bool {
    let Some((index, _total)) = fetch_index_head(url).await else {
        return false;
    };
    let mip = pick_preview_level(&index, PREVIEW_MAX_EDGE).clone();
    let Some(blocks) = fetch_mip_blocks(url, &mip).await else {
        return false;
    };
    if !commit_mip(
        engine,
        terrain_w,
        terrain_h,
        &mip,
        blocks,
        MODE_SINGLE,
        1,
        1.0,
    )
    .await
    {
        return false;
    }
    {
        let mut b = bridge.borrow_mut();
        b.sat_w = mip.width;
        b.sat_h = mip.height;
        b.sat_mode = "single".into();
        b.sat_mips = 1;
    }
    publish(bridge);
    true
}

async fn load_unified_full(
    engine: &EngineHandle,
    url: &str,
    terrain_w: f64,
    terrain_h: f64,
    bridge: &BridgeHandle,
) -> bool {
    let Some(buf) = fetch_bytes(url).await else {
        return false;
    };
    let Ok(index) = parse_tbd_sat(&buf) else {
        return false;
    };
    let max_dim = {
        let g = engine.borrow();
        g.as_ref()
            .map(|e| e.max_texture_dimension_2d())
            .unwrap_or(8192)
    };
    let base = pick_base_level(&index, max_dim) as usize;
    let Some(base_mip) = index.mips.get(base).cloned() else {
        return false;
    };
    let mip_count = (index.mip_count as usize).saturating_sub(base) as u32;
    let webgl2 = {
        let g = engine.borrow();
        g.as_ref().map(|e| e.backend() == "webgl2").unwrap_or(true)
    };

    // Decode all mips ≥ base before taking the engine borrow for begin/write/commit.
    let mut levels: Vec<(u32, TbdSatMip, Vec<(super::tbd_sat::TbdSatTile, Decoded)>)> = Vec::new();
    for (li, mip) in index.mips.iter().enumerate().skip(base) {
        let rel = (li - base) as u32;
        let mut decoded_tiles = Vec::new();
        for tile in &mip.tiles {
            let start = tile.offset as usize;
            let end = start + tile.length as usize;
            if end > buf.len() {
                return false;
            }
            let Some(d) = decode_webp(&buf[start..end], webgl2).await else {
                return false;
            };
            decoded_tiles.push((tile.clone(), d));
        }
        levels.push((rel, mip.clone(), decoded_tiles));
    }

    {
        let mut guard = engine.borrow_mut();
        let Some(e) = guard.as_mut() else {
            return false;
        };
        if e.tex_layer_begin(
            ROLE_BASEMAP,
            0.0,
            0.0,
            terrain_w,
            terrain_h,
            base_mip.width,
            base_mip.height,
            mip_count,
            MODE_UNIFIED,
        )
        .is_err()
        {
            return false;
        }
        for (rel, _mip, tiles) in levels {
            for (tile, d) in tiles {
                if !upload_decoded(e, ROLE_BASEMAP, rel, tile.x, tile.y, d) {
                    return false;
                }
            }
        }
        if e.tex_layer_commit(ROLE_BASEMAP, 1.0_f32, true).is_err() {
            return false;
        }
    }
    {
        let mut b = bridge.borrow_mut();
        b.sat_w = base_mip.width;
        b.sat_h = base_mip.height;
        b.sat_mode = "unified".into();
        b.sat_mips = mip_count;
    }
    publish(bridge);
    true
}

/// T-173 P6/H8 — restore the unified satellite lane as the visible basemap (opacity 1, no
/// texture rebuild). Used when the operator switches the Mission Settings basemap radio back from
/// Map to Satellite.
pub fn show_satellite_basemap(engine: &EngineHandle) {
    if let Some(e) = engine.borrow_mut().as_mut() {
        e.set_lane_opacity(ROLE_BASEMAP, 1.0, true);
    }
}

/// T-173 P6/H8 — load the stylized **Map** cartographic pyramid (`tiles/map/{z}/{x}/{y}.webp`)
/// into the basemap lane as one stitched level. Picks the largest XYZ zoom whose stitched edge
/// (`2^z · 256`) fits the GPU's `maxTextureDimension2D`, decodes every tile, and uploads via the
/// same `tex_layer_*` path the satellite loader uses (single level, MODE_SINGLE). Returns false if
/// the pyramid is absent (tiles not built locally) so the caller can fall back to satellite.
pub async fn load_map_basemap(
    engine: &EngineHandle,
    terrain: &str,
    terrain_w: f64,
    terrain_h: f64,
) -> bool {
    let max_dim = {
        let g = engine.borrow();
        g.as_ref()
            .map(|e| e.max_texture_dimension_2d())
            .unwrap_or(8192)
    };
    // Largest z in [0, 6] with 2^z·256 ≤ max_dim (cap z4 = 4096² — the cartographic source res).
    let mut z: u32 = 0;
    for cand in 0..=4u32 {
        if (1u32 << cand) * 256 <= max_dim {
            z = cand;
        }
    }
    let tiles_per_side = 1u32 << z;
    let stitched = tiles_per_side * 256;
    let webgl2 = {
        let g = engine.borrow();
        g.as_ref().map(|e| e.backend() == "webgl2").unwrap_or(true)
    };

    // Fetch + decode every tile of the chosen level before taking the engine borrow.
    let mut decoded: Vec<(u32, u32, Decoded)> = Vec::new();
    for ty in 0..tiles_per_side {
        for tx in 0..tiles_per_side {
            let url = format!("/map-assets/{terrain}/tiles/map/{z}/{tx}/{ty}.webp");
            let Some(bytes) = fetch_bytes(&url).await else {
                return false; // pyramid not built — caller falls back to satellite
            };
            let Some(d) = decode_webp(&bytes, webgl2).await else {
                return false;
            };
            decoded.push((tx * 256, ty * 256, d));
        }
    }

    let mut guard = engine.borrow_mut();
    let Some(e) = guard.as_mut() else {
        return false;
    };
    if e.tex_layer_begin(
        ROLE_BASEMAP,
        0.0,
        0.0,
        terrain_w,
        terrain_h,
        stitched,
        stitched,
        1,
        MODE_SINGLE,
    )
    .is_err()
    {
        return false;
    }
    for (x, y, d) in decoded {
        if !upload_decoded(e, ROLE_BASEMAP, 0, x, y, d) {
            return false;
        }
    }
    e.tex_layer_commit(ROLE_BASEMAP, 1.0, true).is_ok()
}

/// Load satellite for `terrain`. Preview via Range first; full unified unless `sat=preview`.
///
/// Dev default is preview-only (`localhost` / `127.0.0.1` without `sat=full`) so `make leptos`
/// does not freeze the tab on a 152 MB GET — pass `?sat=full` for the complete mip chain.
pub async fn load_satellite(
    engine: EngineHandle,
    base: &str,
    unified_url: &str,
    terrain_w: f64,
    terrain_h: f64,
    bridge: BridgeHandle,
) {
    let url = if unified_url.starts_with('/') {
        unified_url.to_string()
    } else {
        format!("{base}/{unified_url}")
    };
    let _ = try_preview(&engine, &url, terrain_w, terrain_h, &bridge).await;
    if sat_preview_only() || sat_dev_preview_default() {
        return;
    }
    let _ = load_unified_full(&engine, &url, terrain_w, terrain_h, &bridge).await;
}

/// Local Trunk/dev hosts skip the full-bundle GET unless `?sat=full` is set.
fn sat_dev_preview_default() -> bool {
    let Some(win) = web_sys::window() else {
        return false;
    };
    let Ok(host) = win.location().hostname() else {
        return false;
    };
    let local = host == "localhost" || host == "127.0.0.1" || host.ends_with(".localhost");
    if !local {
        return false;
    }
    let search = win.location().search().unwrap_or_default();
    !search.contains("sat=full")
}
