//! Role: fetch.
//! Position: `streaming/loaders` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use wasm_bindgen::JsCast;

use crate::streaming::bridge::progress::BootEvent;
use crate::streaming::bridge::progress::BootSeg;
use crate::streaming::bridge::progress::STREAM_REPORT_BYTES;

/// Soft-fail byte GET (same-origin `/map-assets`).
pub async fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    let resp = gloo_net::http::Request::get(url).send().await.ok()?;
    if !(200..300).contains(&resp.status()) {
        return None;
    }
    resp.binary().await.ok()
}

/// Two deliberate properties:.
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

/// Fetch text.
pub async fn fetch_text(url: &str) -> Option<String> {
    let resp = gloo_net::http::Request::get(url).send().await.ok()?;
    if !(200..300).contains(&resp.status()) {
        return None;
    }
    resp.text().await.ok()
}

/// Range body.
pub struct RangeBody {
    /// Bytes.
    pub bytes: Vec<u8>,

    /// Total.
    pub total: u64,
}

/// Range outcome.
pub enum RangeOutcome {
    /// Body.
    Body(RangeBody),

    /// `429` — carries the server's `Retry-After` in seconds when it sent one.
    RateLimited { retry_after_s: Option<u64> },

    /// Any other non-206 status, or a transport/parse failure.
    Failed { status: u16 },
}

/// HTTP Range GET, reporting **which way it went** — see [`RangeOutcome`]. Succeeds only on **206**; a 200 (server ignoring Range) is rejected so CI never silently downloads the full 152_713_114 B sat bundle.
pub async fn fetch_range_outcome(url: &str, start: u64, end_inclusive: u64) -> RangeOutcome {
    let Ok(resp) = gloo_net::http::Request::get(url)
        .header("Range", &format!("bytes={start}-{end_inclusive}"))
        .send()
        .await
    else {
        return RangeOutcome::Failed { status: 0 };
    };
    let status = resp.status();
    if status == 429 {
        return RangeOutcome::RateLimited {
            retry_after_s: resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.parse::<u64>().ok()),
        };
    }
    if status != 206 {
        return RangeOutcome::Failed { status };
    }
    let total = resp
        .headers()
        .get("content-range")
        .and_then(|cr| cr.split('/').nth(1)?.parse::<u64>().ok())
        .filter(|&t| t > 0);
    let (Some(total), Ok(bytes)) = (total, resp.binary().await) else {
        return RangeOutcome::Failed { status };
    };
    RangeOutcome::Body(RangeBody { bytes, total })
}
