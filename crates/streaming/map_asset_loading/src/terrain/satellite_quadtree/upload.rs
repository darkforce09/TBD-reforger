//! **Role:** the commit of one mip level into the basemap texture layer.
//! **Position:** `terrain::satellite_quadtree` in `map_asset_loading`; the full chain load calls it
//! per level.
//! **Signals & state:** writes the texture layer through the asset sink.
//! **Invariants:** a level is written whole before the layer is committed.

use super::BrowserAssetSinkHandle;
use super::ROLE_BASEMAP;
use super::TbdSatMip;
use super::TextureLayerSpec;

use super::decode_webp;
use super::upload_decoded;

/// Commit mip.
#[allow(clippy::too_many_arguments)]
pub(super) async fn commit_mip(
    engine: &BrowserAssetSinkHandle,
    terrain_w: f64,
    terrain_h: f64,
    mip: &TbdSatMip,
    blocks: Vec<(satellite_imagery::TbdSatTile, Vec<u8>)>,
    mode: u32,
    mip_count: u32,
    opacity: f64,
) -> bool {
    let webgl2 = {
        let g = engine.borrow();
        g.sink().map(|e| e.backend_is_webgl2()).unwrap_or(true)
    };
    let mut decoded = Vec::with_capacity(blocks.len());
    for (tile, bytes) in &blocks {
        let Some(d) = decode_webp(bytes, webgl2).await else {
            return false;
        };
        decoded.push((tile.clone(), d));
    }
    let mut guard = engine.borrow_mut();
    let Some(e) = guard.sink_mut() else {
        return false;
    };
    let layer = TextureLayerSpec {
        role: ROLE_BASEMAP,
        world_min: [0.0, 0.0],
        world_max: [terrain_w, terrain_h],
        width: mip.width,
        height: mip.height,
        mip_count,
        mode,
    };
    if e.tex_layer_begin(&layer).is_err() {
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
