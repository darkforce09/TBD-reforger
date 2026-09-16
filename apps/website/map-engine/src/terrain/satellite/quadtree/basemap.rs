//! Role: basemap.
//! Position: `terrain/satellite/quadtree` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::BootEvent;
use super::BridgeHandle;
use super::Decoded;
use super::EngineHandle;
use super::MODE_SINGLE;
use super::PREVIEW_MAX_EDGE;
use super::ROLE_BASEMAP;
use super::decode_webp;
use super::fetch_bytes;
use super::load_unified_full;
use super::sat_preview_only;
use super::texture_limit;
use super::try_preview;
use super::upload_decoded;

/// Show satellite basemap.
pub fn show_satellite_basemap(engine: &EngineHandle) {
    if let Some(e) = engine.borrow_mut().as_mut() {
        e.set_lane_opacity(ROLE_BASEMAP, 1.0, true);
    }
}

/// Load map basemap.
pub async fn load_map_basemap(
    engine: &EngineHandle,
    terrain: &str,
    terrain_w: f64,
    terrain_h: f64,
) -> bool {
    let Some(limit) = texture_limit(engine) else {
        crate::diagnostics::platform::console::error!(
            "map basemap: no render engine when the pyramid zoom had to be chosen — refusing to \
             guess a GPU texture limit."
        );
        return false;
    };
    let max_dim = limit.device;

    let mut z: u32 = 0;
    for cand in 0..=4u32 {
        if (1u32 << cand) * 256 <= max_dim {
            z = cand;
        }
    }
    let tiles_per_side = 1u32 << z;
    let stitched = tiles_per_side * 256;
    let webgl2 = {
        let g = engine.borrow();
        g.as_ref().map(|e| e.backend() == "webgl2").unwrap_or(true)
    };

    let mut decoded: Vec<(u32, u32, Decoded)> = Vec::new();
    for ty in 0..tiles_per_side {
        for tx in 0..tiles_per_side {
            let url = format!("/map-assets/{terrain}/tiles/map/{z}/{tx}/{ty}.webp");
            let Some(bytes) = fetch_bytes(&url).await else {
                return false;
            };
            let Some(d) = decode_webp(&bytes, webgl2).await else {
                return false;
            };
            decoded.push((tx * 256, ty * 256, d));
        }
    }

    let mut guard = engine.borrow_mut();
    let Some(e) = guard.as_mut() else {
        return false;
    };
    if e.tex_layer_begin(
        ROLE_BASEMAP,
        0.0,
        0.0,
        terrain_w,
        terrain_h,
        stitched,
        stitched,
        1,
        MODE_SINGLE,
    )
    .is_err()
    {
        return false;
    }
    for (x, y, d) in decoded {
        if !upload_decoded(e, ROLE_BASEMAP, 0, x, y, d) {
            return false;
        }
    }
    e.tex_layer_commit(ROLE_BASEMAP, 1.0, true).is_ok()
}

/// Load satellite for `terrain`. Preview via Range first, then the full unified mip chain (preview -> full progressive) on every host, including `localhost` day-to-day (`cargo xtask mk leptos`), so the editor is sharp instead of stuck on the <=1024 px preview: the coarse preview shows first, then `load_unified_full` replaces it in the background.
pub async fn load_satellite(
    engine: EngineHandle,
    base: &str,
    unified_url: &str,
    terrain_w: f64,
    terrain_h: f64,
    bridge: BridgeHandle,
    report: &dyn Fn(BootEvent),
) {
    let url = if unified_url.starts_with('/') {
        unified_url.to_string()
    } else {
        format!("{base}/{unified_url}")
    };

    let preview_ok = try_preview(&engine, &url, terrain_w, terrain_h, &bridge).await;
    if sat_preview_only() {
        return;
    }

    if !load_unified_full(&engine, &url, terrain_w, terrain_h, &bridge, report).await {
        if preview_ok {
            crate::diagnostics::platform::console::warn!(
                "satellite: the full-resolution basemap did NOT load — the <={PREVIEW_MAX_EDGE} px \
                 preview placeholder is what is on screen."
            );
        } else {
            crate::diagnostics::platform::console::error!(
                "satellite: no basemap loaded at all — neither the preview nor the full mip chain."
            );
        }
    }
}
