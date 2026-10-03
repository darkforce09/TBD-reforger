//! **Role:** the browser fetch of a manifest-declared raw elevation grid, streamed with progress.
//! **Position:** `terrain::elevation` in `map_asset_loading`; the map host's terrain load asks for
//! the raw grid before the PNG.
//! **Signals & state:** none; the fetch state is the call's own.
//! **Invariants:** only a raw block this build reads is fetched; the body must announce its length.

use browser_platform::fetch::ByteProgress;
use terrain_elevation::raw::RawDem;
use terrain_elevation::raw::RawDemSink;
use world_chunks::terrain_manifest::DemRawBlock;

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
    report_every_bytes: u64,
    progress: &dyn Fn(ByteProgress),
) -> Option<terrain_elevation::png::DecodedDem> {
    let block = block.filter(|b| raw_block_is_readable(b))?;
    let raw = load_dem_raw(
        &format!("{base}/{}", block.path),
        report_every_bytes,
        progress,
    )
    .await?;
    Some(terrain_elevation::png::DecodedDem {
        meters: raw.metres_grid(),
        width: raw.width(),
        height: raw.height(),
    })
}

/// Stream `url` into one `Vec<u16>`: the response must announce its `content-length`, which
/// sizes the sink; `progress` sees the bytes as they arrive, at least every `report_every_bytes`.
pub async fn load_dem_raw(
    url: &str,
    report_every_bytes: u64,
    progress: &dyn Fn(ByteProgress),
) -> Option<RawDem> {
    let body = browser_platform::fetch::open_streamed_body(url).await?;
    let total = body.content_length()?;
    let mut sink = RawDemSink::new(total);
    body.read_to_end(report_every_bytes, progress, &mut |chunk| {
        sink.push(chunk).is_ok()
    })
    .await?;
    sink.finish().ok()
}
