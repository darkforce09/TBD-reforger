//! **Role:** the terrain half of the boot: the manifest's terrain blocks, the elevation model
//! load (raw grid or 16-bit PNG), the hillshade upload, and the satellite URL.
//! **Position:** `terrain_load` in `map_streaming_host`; called by
//! [`crate::bootstrap::bootstrap`].
//! **Signals & state:** none; updates the memory budget ledger as buffers come and go.
//! **Invariants:** the metres cache is derived from the source samples with the source's own
//! encoding, so the full-resolution raster and the cache describe the same heights.

use std::cell::Cell;

use browser_platform::fetch::fetch_bytes_streamed;
use map_asset_loading::browser_asset_sink::BrowserAssetSinkHandle;
use map_streaming_model::asset_sink::{TextureLayerSpec, TextureRegion};
use map_streaming_model::memory_budget::Asset;
use terrain_relief::hillshade::build_hillshade_image;

/// Manifest dem.
#[derive(serde::Deserialize)]
pub(crate) struct ManifestDem {
    /// World bounds.
    #[serde(rename = "worldBounds")]
    pub(crate) world_bounds: [f64; 4],

    /// Dem.
    pub(crate) dem: DemInfo,

    /// Tiles.
    pub(crate) tiles: Option<TilesBlock>,

    /// Water.
    #[serde(default)]
    pub(crate) water: Option<serde_json::Value>,
}

/// Dem info.
#[derive(serde::Deserialize)]
pub(crate) struct DemInfo {
    /// Path.
    pub(crate) path: String,

    /// Min m.
    #[serde(rename = "heightRangeMinM")]
    pub(crate) min_m: f64,

    /// Max m.
    #[serde(rename = "heightRangeMaxM")]
    pub(crate) max_m: f64,

    /// Width px.
    #[serde(default, rename = "widthPx")]
    pub(crate) width_px: Option<u32>,

    /// Height px.
    #[serde(default, rename = "heightPx")]
    pub(crate) height_px: Option<u32>,

    /// Raw.
    #[serde(default)]
    pub(crate) raw: Option<world_chunks::terrain_manifest::DemRawBlock>,
}

/// Tiles block.
#[derive(serde::Deserialize)]
pub(crate) struct TilesBlock {
    /// Satellite.
    pub(crate) satellite: Option<SatBlock>,
}

/// Sat block.
#[derive(serde::Deserialize)]
pub(crate) struct SatBlock {
    /// Unified.
    pub(crate) unified: Option<UnifiedBlock>,
}

/// Unified block.
#[derive(serde::Deserialize)]
pub(crate) struct UnifiedBlock {
    /// Url.
    pub(crate) url: Option<String>,

    /// Path.
    pub(crate) path: Option<String>,
}

/// Elevation model and hillshade as the terrain boot receives them.
pub(crate) struct LoadedTerrain {
    /// Row-major `f32` metres, one per source sample.
    pub(crate) meters: Vec<f32>,

    /// Source samples per row.
    pub(crate) width: u32,

    /// Source rows.
    pub(crate) height: u32,

    /// Hillshade texture width.
    pub(crate) hillshade_w: u32,

    /// Hillshade texture height.
    pub(crate) hillshade_h: u32,

    /// The source `u16` raster, present only when the caller asked to keep it.
    pub(crate) full_resolution: Option<terrain_elevation::full_resolution::FullResolutionDem>,
}

/// The elevation model as its `u16` samples with their encoding, plus the `f32` metres cache
/// derived from them.
type DemSource = (
    terrain_elevation::png::DecodedDem,
    Vec<u16>,
    terrain_elevation::full_resolution::SampleEncoding,
);

/// Turns a streamed fetch's [`browser_platform::fetch::ByteProgress`] into the boot bar's events
/// for `segment`: the first report (zero bytes, before the body) is the segment's byte budget, the
/// announced `content-length` or 0 without one; every later report is the bytes that arrived
/// since the previous one.
fn boot_byte_progress<'report>(
    segment: map_streaming_model::boot_progress::BootSeg,
    report: &'report dyn Fn(map_streaming_model::boot_progress::BootEvent),
) -> impl Fn(browser_platform::fetch::ByteProgress) + 'report {
    use map_streaming_model::boot_progress::BootEvent;
    let previously_received: Cell<Option<u64>> = Cell::new(None);
    move |progress| {
        match previously_received.get() {
            None => report(BootEvent::Budget(segment, progress.total.unwrap_or(0))),
            Some(previous) => report(BootEvent::Done(
                segment,
                progress.received.saturating_sub(previous),
            )),
        }
        previously_received.set(Some(progress.received));
    }
}

/// Stream the manifest-declared raw grid when this build reads its encoding; `None` sends the
/// caller to the PNG.
async fn load_declared_raw_samples(
    base: &str,
    manifest: &ManifestDem,
    report: &dyn Fn(map_streaming_model::boot_progress::BootEvent),
) -> Option<DemSource> {
    let block = manifest
        .dem
        .raw
        .as_ref()
        .filter(|b| map_asset_loading::terrain::elevation::loader::raw_block_is_readable(b))?;
    let raw = map_asset_loading::terrain::elevation::loader::load_dem_raw(
        &format!("{base}/{}", block.path),
        map_streaming_model::boot_progress::STREAM_REPORT_BYTES,
        &boot_byte_progress(map_streaming_model::boot_progress::BootSeg::Terrain, report),
    )
    .await?;
    let decoded = terrain_elevation::png::DecodedDem {
        meters: raw.metres_grid(),
        width: raw.width(),
        height: raw.height(),
    };
    let encoding = terrain_elevation::full_resolution::SampleEncoding {
        offset_m: f64::from(raw.header.offset_m),
        scale_m: f64::from(raw.header.scale_m),
    };
    Some((decoded, raw.samples, encoding))
}

/// Decode the 16-bit PNG into its samples, the `uint16-linear` encoding the manifest declares,
/// and the metres cache.
fn decode_png_source(bytes: &[u8], manifest: &ManifestDem) -> Option<DemSource> {
    let (raster, width, height) = terrain_elevation::png::decode_png_gray16(bytes).ok()?;
    let meters =
        terrain_elevation::sampling::meters_cache(&raster, manifest.dem.min_m, manifest.dem.max_m);
    let encoding = terrain_elevation::full_resolution::SampleEncoding::linear_range(
        manifest.dem.min_m,
        manifest.dem.max_m,
    );
    let decoded = terrain_elevation::png::DecodedDem {
        meters,
        width,
        height,
    };
    Some((decoded, raster, encoding))
}

/// Build the hillshade from the metres cache and upload it as texture lane 1 over the
/// manifest's world bounds; answers the texture size.
fn upload_hillshade(
    engine: &BrowserAssetSinkHandle,
    manifest: &ManifestDem,
    dem: &terrain_elevation::png::DecodedDem,
) -> Option<(u32, u32)> {
    map_asset_loading::live_memory_budget::set_held(
        Asset::Dem,
        dem.meters.len() as u64 * std::mem::size_of::<f32>() as u64,
    );
    let hs = build_hillshade_image(&dem.meters, dem.width as usize, dem.height as usize);
    if hs.data.is_empty() || hs.w == 0 || hs.h == 0 {
        return None;
    }

    let hs_bytes = hs.data.len() as u64;
    map_asset_loading::live_memory_budget::set_held(Asset::Hillshade, hs_bytes);
    let [min_x, min_y, max_x, max_y] = manifest.world_bounds;
    let (w, h) = (hs.w as u32, hs.h as u32);
    {
        let mut guard = engine.borrow_mut();
        let e = guard.sink_mut()?;
        let layer = TextureLayerSpec {
            role: 1,
            world_min: [min_x, min_y],
            world_max: [max_x, max_y],
            width: w,
            height: h,
            mip_count: 1,
            mode: 3,
        };
        e.tex_layer_begin(&layer).ok()?;
        let region = TextureRegion {
            role: 1,
            mip: 0,
            x: 0,
            y: 0,
            width: w,
            height: h,
        };
        e.tex_layer_write_rgba(region, &hs.data).ok()?;
        e.tex_layer_commit(1, 0.4, true).ok()?;
    }
    map_asset_loading::live_memory_budget::release(Asset::Hillshade, hs_bytes);
    Some((w, h))
}

/// Load the elevation model (the declared raw grid first, else the 16-bit PNG through the
/// measured streamed fetch), upload its hillshade, and keep the source raster as a
/// [`terrain_elevation::full_resolution::FullResolutionDem`] when
/// `keep_full_resolution` is set.
pub(crate) async fn load_dem_and_hillshade(
    engine: &BrowserAssetSinkHandle,
    base: &str,
    manifest: &ManifestDem,
    report: &dyn Fn(map_streaming_model::boot_progress::BootEvent),
    keep_full_resolution: bool,
) -> Option<LoadedTerrain> {
    use map_streaming_model::boot_progress::BootSeg;
    let (dem, samples, encoding) = match load_declared_raw_samples(base, manifest, report).await {
        Some(source) => source,
        None => {
            let dem_bytes = fetch_bytes_streamed(
                &format!("{base}/{}", manifest.dem.path),
                map_streaming_model::boot_progress::STREAM_REPORT_BYTES,
                &boot_byte_progress(BootSeg::Terrain, report),
            )
            .await?;
            map_asset_loading::live_memory_budget::hold(Asset::Dem, dem_bytes.len() as u64);
            let decoded = decode_png_source(&dem_bytes, manifest);
            map_asset_loading::live_memory_budget::release(Asset::Dem, dem_bytes.len() as u64);
            decoded?
        }
    };
    let full_resolution = if keep_full_resolution {
        terrain_elevation::full_resolution::FullResolutionDem::new(
            samples,
            dem.width,
            dem.height,
            encoding,
            terrain_elevation::full_resolution::RasterFootprint::from_world_bounds(
                manifest.world_bounds,
            ),
        )
    } else {
        drop(samples);
        None
    };
    let (hillshade_w, hillshade_h) = upload_hillshade(engine, manifest, &dem)?;
    Some(LoadedTerrain {
        meters: dem.meters,
        width: dem.width,
        height: dem.height,
        hillshade_w,
        hillshade_h,
        full_resolution,
    })
}

/// Sat url from.
pub(crate) fn sat_url_from(manifest: &ManifestDem, base: &str) -> Option<(String, f64, f64)> {
    let u = manifest
        .tiles
        .as_ref()?
        .satellite
        .as_ref()?
        .unified
        .as_ref()?;
    let url = u
        .url
        .clone()
        .or_else(|| u.path.as_ref().map(|p| format!("{base}/{p}")))?;
    let [_, _, max_x, max_y] = manifest.world_bounds;
    Some((url, max_x, max_y))
}
