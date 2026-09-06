//! T-935.4 — the DEM boot path: `dem/elevation.dem` (`TBDE`) streamed straight into its final
//! `Vec<u16>`, with the 16-bit PNG kept as the fallback until T-935.13 flips the manifest.
//!
//! # What this replaces
//!
//! The PNG path ([`load_dem_and_hillshade`](super::load_dem_and_hillshade)) moves the everon DEM
//! through four full-grid buffers before the first sample is usable: the 71.9 MB body
//! [`fetch_bytes_streamed`](super::fetch::fetch_bytes_streamed) accumulates, the `png` crate's
//! 81.9 MB output buffer, the 81.9 MB `u16` raster `decode_png_gray16` builds from it, and the
//! 163.8 MB `f32` metres cache. Inflate runs synchronously on wasm's single thread, so the whole
//! decode is a frame the browser cannot paint.
//!
//! The raw file is 81.9 MB — 10 MB *larger* on the wire — and that is the trade: no inflate at all,
//! and the bytes land in the `Vec<u16>` the store keeps as they arrive. There is exactly one
//! allocation on this path.
//!
//! # Why this streams itself instead of calling `fetch_bytes_streamed`
//!
//! `fetch_bytes_streamed` returns a `Vec<u8>`, and turning one into a `Vec<u16>` is a second
//! 81.9 MB allocation plus a full copy — the thing this slice exists to remove (ticket: *"streamed
//! bytes land in the final `Vec<u16>`, no intermediate copies"*). So the loop below is the same
//! `ReadableStream` loop with the same `content-length` budget and the same
//! [`STREAM_REPORT_BYTES`] coalescing, writing into [`RawDemSink`] instead of a byte vector. It
//! reports the same events in the same order, so the boot bar cannot tell the two paths apart.
//!
//! # `content-length` is required here
//!
//! [`RawDemSink`] checks the header's declared `width * height` against the length the response
//! already announced *before* allocating, which is what stops a corrupt or hostile header asking
//! for 8.6 GB. A response with no `content-length` therefore takes the PNG path rather than
//! guessing — the API serves the header for `/map-assets` (it is what makes the terrain segment
//! determinate at all, T-628), so this is a degradation, not a normal outcome.

use map_engine_core::dem::raw::{RawDem, RawDemSink};
use map_engine_core::world::DemRawBlock;
use wasm_bindgen::JsCast;

use crate::editor::mission_editor::boot_progress::{BootEvent, BootSeg, STREAM_REPORT_BYTES};

/// The `dem.raw.encoding` this build reads (spec §5). A block naming anything else describes a
/// format this loader does not implement, and reading it anyway is how you draw a map made of
/// garbage that never once errors — the same reasoning as
/// [`ObjectsBinaryBlock::matches_this_build`](map_engine_core::world::ObjectsBinaryBlock::matches_this_build).
const TBDE_ENCODING_V1: &str = "tbde-v1";

/// Does this `dem.raw` block describe a file this build can read?
///
/// `DemRawBlock` is `serde(default)`, so a hand-edited manifest missing `encoding` deserialises
/// with an empty string rather than failing — which must mean "fall back to the PNG", not "assume
/// v1".
#[must_use]
pub fn raw_block_is_readable(block: &DemRawBlock) -> bool {
    !block.path.is_empty() && block.encoding == TBDE_ENCODING_V1
}

/// Stream `url` into one `Vec<u16>`.
///
/// Returns `None` — leaving the caller to use the PNG — on any non-2xx, a missing
/// `content-length`, a body-less response, a transport error, or a `TBDE` frame that does not
/// validate. Every one of those is a reason to fall back, never a reason to draw a partial DEM.
pub async fn load_dem_raw(url: &str, seg: BootSeg, report: &dyn Fn(BootEvent)) -> Option<RawDem> {
    let resp = gloo_net::http::Request::get(url).send().await.ok()?;
    if !(200..300).contains(&resp.status()) {
        return None;
    }
    // Not `unwrap_or(0)` as in `fetch_bytes_streamed`: there the header is only a progress budget,
    // here it is the allocation guard. See the module docs.
    let total = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.parse::<u64>().ok())?;
    report(BootEvent::Budget(seg, total));

    let body = resp.body()?;
    let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().unchecked_into();
    let mut sink = RawDemSink::new(total);
    let mut unreported: u64 = 0;
    loop {
        let chunk = wasm_bindgen_futures::JsFuture::from(reader.read())
            .await
            .ok()?;
        let done = js_sys::Reflect::get(&chunk, &"done".into())
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        if done {
            break;
        }
        let arr: js_sys::Uint8Array = js_sys::Reflect::get(&chunk, &"value".into())
            .ok()?
            .unchecked_into();
        // `to_vec` is the one copy the JS boundary forces — a `Uint8Array` lives in the JS heap and
        // has to cross into linear memory before it can be read. It is per-chunk (tens of KB), not
        // per-grid, and it is freed before the next `read()`.
        sink.push(&arr.to_vec()).ok()?;
        unreported += u64::from(arr.length());
        if unreported >= STREAM_REPORT_BYTES {
            report(BootEvent::Done(seg, unreported));
            unreported = 0;
        }
    }
    if unreported > 0 {
        report(BootEvent::Done(seg, unreported));
    }
    sink.finish().ok()
}
