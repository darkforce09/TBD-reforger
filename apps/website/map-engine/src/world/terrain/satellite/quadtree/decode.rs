//! Role: decode.
//! Position: `world/terrain/satellite/quadtree` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::JsFuture;
use wasm_bindgen::JsCast;

/// Decoded.
pub(super) enum Decoded {
    /// Bitmap.
    Bitmap(web_sys::ImageBitmap),

    /// Rgba.
    Rgba { w: u32, h: u32, rgba: Vec<u8> },
}

/// Decode webp.
pub(super) async fn decode_webp(bytes: &[u8], webgl2: bool) -> Option<Decoded> {
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

/// Upload decoded.
pub(super) fn upload_decoded(
    engine: &mut crate::core::context::state::RenderEngine,
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
