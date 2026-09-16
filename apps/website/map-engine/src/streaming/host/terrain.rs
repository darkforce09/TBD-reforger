//! Role: terrain.
//! Position: `streaming/host` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

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

/// Load dem and hillshade.
pub(super) async fn load_dem_and_hillshade(
    engine: &EngineHandle,
    base: &str,
    manifest: &ManifestDem,
    report: &dyn Fn(crate::streaming::bridge::progress::BootEvent),
) -> Option<(Vec<f32>, u32, u32, u32, u32)> {
    use crate::streaming::bridge::progress::BootSeg;
    let dem = match crate::world::terrain::dem::loader::load_declared_raw(
        base,
        manifest.dem.raw.as_ref(),
        report,
    )
    .await
    {
        Some(raw) => raw,
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
            let decoded = decode_png_to_meters(&dem_bytes, manifest.dem.min_m, manifest.dem.max_m);
            crate::streaming::memory::budget::release(
                crate::streaming::memory::budget::Asset::Dem,
                dem_bytes.len() as u64,
            );
            decoded.ok()?
        }
    };

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
    Some((dem.meters, dem.width, dem.height, w, h))
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
