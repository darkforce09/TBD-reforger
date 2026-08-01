//! Shared HTTP helpers for the map-asset host (full GET + streamed GET + Range).

use wasm_bindgen::JsCast;

use crate::mission_editor::boot_progress::{BootEvent, BootSeg, STREAM_REPORT_BYTES};

/// Soft-fail byte GET (same-origin `/map-assets`).
pub async fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    let resp = gloo_net::http::Request::get(url).send().await.ok()?;
    if !(200..300).contains(&resp.status()) {
        return None;
    }
    resp.binary().await.ok()
}

/// T-628 — the same GET, but **measured**: the budget comes off `content-length` before a body byte
/// arrives, and progress comes off the response's `ReadableStream` as the bytes actually land.
///
/// This is what makes the terrain segment determinate. The DEM is a single 71,911,548 B PNG that
/// `fetch_bytes` returns in one lump, so a bar fed by it sits at 0% for the whole download and then
/// snaps — the exact "conveys nothing" failure the sweep had. Reading the stream costs one extra
/// `get_reader()` and no extra request, no extra connection and no extra round trip, which is why
/// it is preferred here over splitting the file into Range spans the way the satellite has to.
///
/// Two deliberate properties:
///
/// * **`content-length` is a budget, never progress.** [`BootEvent::Done`] only ever carries bytes
///   that came out of `reader.read()`. If the header is missing the segment simply has no budget
///   and stays at 0 until [`BootEvent::Finish`] — it does not get a guess.
/// * **Reports are coalesced to [`STREAM_REPORT_BYTES`].** The socket hands back 16–64 KB at a time
///   and the bar cannot render that finely; see that constant.
///
/// A body-less response (nothing in the browser gives one for a 200 with content, but the type is
/// `Option`) falls back to the buffered read and reports the whole length once — degraded, still
/// honest.
pub async fn fetch_bytes_streamed(
    url: &str,
    seg: BootSeg,
    report: &dyn Fn(BootEvent),
) -> Option<Vec<u8>> {
    let resp = gloo_net::http::Request::get(url).send().await.ok()?;
    if !(200..300).contains(&resp.status()) {
        return None;
    }
    let budget = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    report(BootEvent::Budget(seg, budget));

    let Some(body) = resp.body() else {
        let bytes = resp.binary().await.ok()?;
        report(BootEvent::Done(seg, bytes.len() as u64));
        return Some(bytes);
    };
    let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().unchecked_into();
    let mut out: Vec<u8> = Vec::with_capacity(usize::try_from(budget).unwrap_or(0));
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
        let Ok(value) = js_sys::Reflect::get(&chunk, &"value".into()) else {
            return None;
        };
        let arr: js_sys::Uint8Array = value.unchecked_into();
        let at = out.len();
        out.resize(at + arr.length() as usize, 0);
        arr.copy_to(&mut out[at..]);
        unreported += u64::from(arr.length());
        if unreported >= STREAM_REPORT_BYTES {
            report(BootEvent::Done(seg, unreported));
            unreported = 0;
        }
    }
    if unreported > 0 {
        report(BootEvent::Done(seg, unreported));
    }
    Some(out)
}

pub async fn fetch_text(url: &str) -> Option<String> {
    let resp = gloo_net::http::Request::get(url).send().await.ok()?;
    if !(200..300).contains(&resp.status()) {
        return None;
    }
    resp.text().await.ok()
}

pub struct RangeBody {
    pub bytes: Vec<u8>,
    pub total: u64,
}

/// HTTP Range GET. Succeeds only on **206**; a 200 (server ignoring Range) is rejected so CI
/// never silently downloads the full 152_713_114 B sat bundle.
pub async fn fetch_range(url: &str, start: u64, end_inclusive: u64) -> Option<RangeBody> {
    let resp = gloo_net::http::Request::get(url)
        .header("Range", &format!("bytes={start}-{end_inclusive}"))
        .send()
        .await
        .ok()?;
    if resp.status() != 206 {
        return None;
    }
    let total = resp
        .headers()
        .get("content-range")
        .and_then(|cr| cr.split('/').nth(1)?.parse::<u64>().ok())
        .filter(|&t| t > 0)?;
    let bytes = resp.binary().await.ok()?;
    Some(RangeBody { bytes, total })
}
