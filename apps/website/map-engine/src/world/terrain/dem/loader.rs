//! Role: loader.
//! Position: `world/terrain/dem` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::streaming::loaders::manifest::DemRawBlock;
use crate::world::terrain::dem::raw::RawDem;
use crate::world::terrain::dem::raw::RawDemSink;
use wasm_bindgen::JsCast;

use crate::streaming::bridge::progress::BootEvent;
use crate::streaming::bridge::progress::BootSeg;
use crate::streaming::bridge::progress::STREAM_REPORT_BYTES;

const TBDE_ENCODING_V1: &str = "tbde-v1";

/// Does this `dem.raw` block describe a file this build can read?.
#[must_use]
pub fn raw_block_is_readable(block: &DemRawBlock) -> bool {
    !block.path.is_empty() && block.encoding == TBDE_ENCODING_V1
}

/// The whole raw arm of the DEM boot path: take the `dem.raw` block *if* the manifest declares one this build can read, stream it, and hand back the `DecodedDem` the PNG path also produces.
pub async fn load_declared_raw(
    base: &str,
    block: Option<&DemRawBlock>,
    report: &dyn Fn(BootEvent),
) -> Option<crate::world::terrain::dem::png::DecodedDem> {
    let block = block.filter(|b| raw_block_is_readable(b))?;
    let raw = load_dem_raw(&format!("{base}/{}", block.path), BootSeg::Terrain, report).await?;
    Some(crate::world::terrain::dem::png::DecodedDem {
        meters: raw.metres_grid(),
        width: raw.width(),
        height: raw.height(),
    })
}

/// Stream `url` into one `Vec<u16>`.
pub async fn load_dem_raw(url: &str, seg: BootSeg, report: &dyn Fn(BootEvent)) -> Option<RawDem> {
    let resp = gloo_net::http::Request::get(url).send().await.ok()?;
    if !(200..300).contains(&resp.status()) {
        return None;
    }

    let total = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.parse::<u64>().ok())?;

    let body = resp.body()?;
    report(BootEvent::Budget(seg, total));
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
