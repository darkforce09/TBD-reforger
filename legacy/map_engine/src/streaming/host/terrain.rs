//! **Role:** the terrain half of the boot: the manifest's terrain blocks, the elevation model
//! load (raw grid or 16-bit PNG), the hillshade upload, and the satellite URL.
//! **Position:** `streaming/host`; called by [`super::bootstrap::bootstrap`].
//! **Signals & state:** none; updates the memory budget ledger as buffers come and go.
//! **Invariants:** the metres cache is derived from the source samples with the source's own
//! encoding, so the full-resolution raster and the cache describe the same heights.

use super::*;

/// Manifest dem.
#[derive(serde::Deserialize)]
pub(super) struct ManifestDem {
    /// World bounds.
    #[serde(rename = "worldBounds")]
    pub(super) world_bounds: [f64; 4],

    /// Dem.
    pub(super) dem: DemInfo,

    /// Tiles.
    pub(super) tiles: Option<TilesBlock>,

    /// Water.
    #[serde(default)]
    pub(super) water: Option<serde_json::Value>,
}

/// Dem info.
#[derive(serde::Deserialize)]
pub(super) struct DemInfo {
    /// Path.
    pub(super) path: String,

    /// Min m.
    #[serde(rename = "heightRangeMinM")]
    pub(super) min_m: f64,

    /// Max m.
    #[serde(rename = "heightRangeMaxM")]
    pub(super) max_m: f64,

    /// Width px.
    #[serde(default, rename = "widthPx")]
    pub(super) width_px: Option<u32>,

    /// Height px.
    #[serde(default, rename = "heightPx")]
    pub(super) height_px: Option<u32>,

    /// Raw.
    #[serde(default)]
    pub(super) raw: Option<crate::streaming::loaders::manifest::DemRawBlock>,
}

/// Tiles block.
#[derive(serde::Deserialize)]
pub(super) struct TilesBlock {
    /// Satellite.
    pub(super) satellite: Option<SatBlock>,
}

/// Sat block.
#[derive(serde::Deserialize)]
pub(super) struct SatBlock {
    /// Unified.
    pub(super) unified: Option<UnifiedBlock>,
}

/// Unified block.
#[derive(serde::Deserialize)]
pub(super) struct UnifiedBlock {
    /// Url.
    pub(super) url: Option<String>,

    /// Path.
    pub(super) path: Option<String>,
}

/// Elevation model and hillshade as the terrain boot receives them.
pub(super) struct LoadedTerrain {
    /// Row-major `f32` metres, one per source sample.
    pub(super) meters: Vec<f32>,

    /// Source samples per row.
    pub(super) width: u32,

    /// Source rows.
    pub(super) height: u32,

    /// Hillshade texture width.
    pub(super) hillshade_w: u32,

    /// Hillshade texture height.
    pub(super) hillshade_h: u32,

    /// The source `u16` raster, present only when the caller asked to keep it.
    pub(super) full_resolution:
        Option<crate::world::terrain::dem::full_resolution::FullResolutionDem>,
}

/// The elevation model as its `u16` samples with their encoding, plus the `f32` metres cache
/// derived from them.
type DemSource = (
    crate::world::terrain::dem::png::DecodedDem,
    Vec<u16>,
    crate::world::terrain::dem::full_resolution::SampleEncoding,
);

/// Stream the manifest-declared raw grid when this build reads its encoding; `None` sends the
/// caller to the PNG.
async fn load_declared_raw_samples(
    base: &str,
    manifest: &ManifestDem,
    report: &dyn Fn(crate::streaming::bridge::progress::BootEvent),
) -> Option<DemSource> {
    let block = manifest
        .dem
        .raw
        .as_ref()
        .filter(|b| crate::world::terrain::dem::loader::raw_block_is_readable(b))?;
    let raw = crate::world::terrain::dem::loader::load_dem_raw(
        &format!("{base}/{}", block.path),
        crate::streaming::bridge::progress::BootSeg::Terrain,
        report,
    )
    .await?;
    let decoded = crate::world::terrain::dem::png::DecodedDem {
        meters: raw.metres_grid(),
        width: raw.width(),
        height: raw.height(),
    };
    let encoding = crate::world::terrain::dem::full_resolution::SampleEncoding {
        offset_m: f64::from(raw.header.offset_m),
        scale_m: f64::from(raw.header.scale_m),
    };
    Some((decoded, raw.samples, encoding))
}

/// Decode the 16-bit PNG into its samples, the `uint16-linear` encoding the manifest declares,
/// and the metres cache.
fn decode_png_source(bytes: &[u8], manifest: &ManifestDem) -> Option<DemSource> {
    let (raster, width, height) = crate::world::terrain::dem::png::decode_png_gray16(bytes).ok()?;
    let meters = crate::world::terrain::dem::sampling::meters_cache(
        &raster,
        manifest.dem.min_m,
        manifest.dem.max_m,
    );
    let encoding = crate::world::terrain::dem::full_resolution::SampleEncoding::linear_range(
        manifest.dem.min_m,
        manifest.dem.max_m,
    );
    let decoded = crate::world::terrain::dem::png::DecodedDem {
        meters,
        width,
        height,
    };
    Some((decoded, raster, encoding))
}

/// Build the hillshade from the metres cache and upload it as texture lane 1 over the
/// manifest's world bounds; answers the texture size.
fn upload_hillshade(
    engine: &EngineHandle,
    manifest: &ManifestDem,
    dem: &crate::world::terrain::dem::png::DecodedDem,
) -> Option<(u32, u32)> {
    crate::streaming::memory::budget::set_held(
        crate::streaming::memory::budget::Asset::Dem,
        dem.meters.len() as u64 * std::mem::size_of::<f32>() as u64,
    );
    let hs = build_hillshade_image(&dem.meters, dem.width as usize, dem.height as usize);
    if hs.data.is_empty() || hs.w == 0 || hs.h == 0 {
        return None;
    }

    let hs_bytes = hs.data.len() as u64;
    crate::streaming::memory::budget::set_held(
        crate::streaming::memory::budget::Asset::Hillshade,
        hs_bytes,
    );
    let [min_x, min_y, max_x, max_y] = manifest.world_bounds;
    let (w, h) = (hs.w as u32, hs.h as u32);
    {
        let mut guard = engine.borrow_mut();
        let e = guard.as_mut()?;
        e.tex_layer_begin(1, min_x, min_y, max_x, max_y, w, h, 1, 3)
            .ok()?;
        e.tex_layer_write_rgba(1, 0, 0, 0, w, h, &hs.data).ok()?;
        e.tex_layer_commit(1, 0.4, true).ok()?;
    }
    crate::streaming::memory::budget::release(
        crate::streaming::memory::budget::Asset::Hillshade,
        hs_bytes,
    );
    Some((w, h))
}

/// Load the elevation model (the declared raw grid first, else the 16-bit PNG through the
/// measured streamed fetch), upload its hillshade, and keep the source raster as a
/// [`crate::world::terrain::dem::full_resolution::FullResolutionDem`] when
/// `keep_full_resolution` is set.
pub(super) async fn load_dem_and_hillshade(
    engine: &EngineHandle,
    base: &str,
    manifest: &ManifestDem,
    report: &dyn Fn(crate::streaming::bridge::progress::BootEvent),
    keep_full_resolution: bool,
) -> Option<LoadedTerrain> {
    use crate::streaming::bridge::progress::BootSeg;
    let (dem, samples, encoding) = match load_declared_raw_samples(base, manifest, report).await {
        Some(source) => source,
        None => {
            let dem_bytes = fetch_bytes_streamed(
                &format!("{base}/{}", manifest.dem.path),
                BootSeg::Terrain,
                report,
            )
            .await?;
            crate::streaming::memory::budget::hold(
                crate::streaming::memory::budget::Asset::Dem,
                dem_bytes.len() as u64,
            );
            let decoded = decode_png_source(&dem_bytes, manifest);
            crate::streaming::memory::budget::release(
                crate::streaming::memory::budget::Asset::Dem,
                dem_bytes.len() as u64,
            );
            decoded?
        }
    };
    let full_resolution = if keep_full_resolution {
        crate::world::terrain::dem::full_resolution::FullResolutionDem::new(
            samples,
            dem.width,
            dem.height,
            encoding,
            crate::world::terrain::dem::full_resolution::RasterFootprint::from_world_bounds(
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
pub(super) fn sat_url_from(manifest: &ManifestDem, base: &str) -> Option<(String, f64, f64)> {
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
