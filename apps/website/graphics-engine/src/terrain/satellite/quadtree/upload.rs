//! Role: upload.
//! Position: `terrain/satellite/quadtree` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::EngineHandle;
use super::ROLE_BASEMAP;
use super::TbdSatMip;

use super::decode_webp;
use super::upload_decoded;

/// Commit mip.
#[allow(clippy::too_many_arguments)]
pub(super) async fn commit_mip(
    engine: &EngineHandle,
    terrain_w: f64,
    terrain_h: f64,
    mip: &TbdSatMip,
    blocks: Vec<(crate::terrain::satellite::streamer::TbdSatTile, Vec<u8>)>,
    mode: u32,
    mip_count: u32,
    opacity: f64,
) -> bool {
    let webgl2 = {
        let g = engine.borrow();
        g.as_ref().map(|e| e.backend() == "webgl2").unwrap_or(true)
    };
    let mut decoded = Vec::with_capacity(blocks.len());
    for (tile, bytes) in &blocks {
        let Some(d) = decode_webp(bytes, webgl2).await else {
            return false;
        };
        decoded.push((tile.clone(), d));
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
        mip.width,
        mip.height,
        mip_count,
        mode,
    )
    .is_err()
    {
        return false;
    }
    for (tile, d) in decoded {
        if !upload_decoded(e, ROLE_BASEMAP, 0, tile.x, tile.y, d) {
            return false;
        }
    }
    e.tex_layer_commit(ROLE_BASEMAP, opacity as f32, true)
        .is_ok()
}
