//! Role: atlas.
//! Position: `streaming/loaders/world_loader` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::AtlasUpload;
use super::BootEvent;
use super::BootSeg;
use super::BridgeHandle;
use super::EngineHandle;
use super::JsFuture;
use super::WorldHost;
use super::fetch_bytes;
use super::fetch_text;
use wasm_bindgen::JsCast;

/// Canonical atlas webp value.
pub(super) const ATLAS_WEBP: &str = "/map-assets/glyphs/atlas/world-glyphs.webp";

/// Canonical atlas json value.
pub(super) const ATLAS_JSON: &str = "/map-assets/glyphs/atlas/world-glyphs.json";

impl WorldHost {
    /// Ensure atlas.
    pub(super) fn ensure_atlas(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) {
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
}

/// Load glyph atlas.
pub(super) async fn load_glyph_atlas(report: &dyn Fn(BootEvent)) -> Option<AtlasUpload> {
    let json_txt = fetch_text(ATLAS_JSON).await;
    report(BootEvent::Done(BootSeg::World, 1));
    let json: serde_json::Value = serde_json::from_str(&json_txt?).ok()?;
    let icons = json.get("icons")?.as_object()?;
    let mut keys: Vec<String> = icons.keys().cloned().collect();
    keys.sort();
    let webp = fetch_bytes(ATLAS_WEBP).await;
    report(BootEvent::Done(BootSeg::World, 1));
    let (w, h, rgba) = decode_webp_rgba(&webp?).await?;
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

/// Decode webp rgba.
pub(super) async fn decode_webp_rgba(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
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
