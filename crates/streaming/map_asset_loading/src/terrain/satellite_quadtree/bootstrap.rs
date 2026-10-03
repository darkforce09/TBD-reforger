//! **Role:** `load_unified_full`: the full mip chain of a unified satellite container under the
//! memory budget's floor.
//! **Position:** `terrain::satellite_quadtree` in `map_asset_loading`; `load_satellite` calls it.
//! **Signals & state:** claims the satellite floor on the live budget; writes the texture layer
//! level by level.
//! **Invariants:** the finest level is the one the budget and the GPU limit allow.

use super::BootEvent;
use super::BootSeg;
use super::BridgeHandle;
use super::BrowserAssetSinkHandle;
use super::Decoded;
use super::MODE_UNIFIED;
use super::PREVIEW_MAX_EDGE;
use super::ROLE_BASEMAP;
use super::SAT_CHUNK_BYTES;
use super::TbdSatTile;
use super::TextureLayerSpec;
use super::decode_webp;
use super::fetch_index_head;
use super::fetch_tiles;
use super::pick_base_level_for_limit;
use super::publish;
use super::report_chosen_level;
use super::split_range;
use super::texture_limit;
use super::upload_decoded;

/// Load unified full.
pub(super) async fn load_unified_full(
    engine: &BrowserAssetSinkHandle,
    url: &str,
    terrain_w: f64,
    terrain_h: f64,
    bridge: &BridgeHandle,
    report: &dyn Fn(BootEvent),
) -> bool {
    let Some((index, file_size)) = fetch_index_head(url, true).await else {
        browser_platform::console_error!("satellite: could not read the .tbd-sat index from {url}");
        return false;
    };

    let limit = texture_limit(engine);
    let Some(base) = pick_base_level_for_limit(&index, limit.map(|l| l.device)) else {
        browser_platform::console_error!(
            "satellite: no render engine when the basemap level had to be chosen — refusing to \
             guess a GPU texture limit. The <={PREVIEW_MAX_EDGE} px preview is what is on screen."
        );
        return false;
    };
    let base = base as usize;

    let level_bytes: Vec<map_streaming_model::memory_budget::LevelBytes> = index
        .mips
        .iter()
        .map(|m| map_streaming_model::memory_budget::LevelBytes {
            width: m.width,
            height: m.height,
            compressed: m.tiles.iter().map(|t| t.length).sum(),
        })
        .collect();
    let (budget_mib, held_mib) = crate::live_memory_budget::with_ledger(|l| {
        (
            l.budget() / map_streaming_model::memory_budget::MIB,
            l.held_total() / map_streaming_model::memory_budget::MIB,
        )
    });
    let walk = crate::live_memory_budget::claim_satellite_floor(&level_bytes, base);
    for (level, bytes, decision) in &walk.rejected {
        browser_platform::console_warn!(
            "satellite: memory budget {decision:?}d level {level} ({} MiB resident — RGBA plus \
             tile bodies, held together across the decode) — raising the mip floor by one. \
             Budget {budget_mib} MiB, {held_mib} MiB already held by the other world assets.",
            bytes / map_streaming_model::memory_budget::MIB
        );
    }
    let base = walk.base;
    let Some(base_mip) = index.mips.get(base).cloned() else {
        browser_platform::console_error!(
            "satellite: index has no mip at the chosen base level {base}"
        );
        return false;
    };
    if let Some(limit) = limit {
        report_chosen_level(&index, base, limit);
    }
    let mip_count = (index.mip_count as usize).saturating_sub(base) as u32;
    let webgl2 = {
        let g = engine.borrow();
        g.sink().map(|e| e.backend_is_webgl2()).unwrap_or(true)
    };

    let plan: Vec<(u32, TbdSatTile)> = index
        .mips
        .iter()
        .enumerate()
        .skip(base)
        .flat_map(|(li, mip)| {
            let rel = (li - base) as u32;
            mip.tiles.iter().cloned().map(move |t| (rel, t))
        })
        .collect();
    let tiles: Vec<TbdSatTile> = plan.iter().map(|(_, t)| t.clone()).collect();

    let total: u64 = tiles.iter().map(|t| t.length).sum();

    let req_count: usize = tiles
        .iter()
        .map(|t| split_range(t.offset, t.length, SAT_CHUNK_BYTES).len())
        .sum();
    report(BootEvent::Budget(BootSeg::Satellite, total));
    let Some(bodies) = fetch_tiles(url, file_size, &tiles, |n| {
        report(BootEvent::Done(BootSeg::Satellite, n));
    })
    .await
    else {
        browser_platform::console_error!(
            "satellite: the Range fetch of {} tiles ({total} B) from level {base} down did not \
             complete",
            tiles.len()
        );
        return false;
    };

    let mut levels: Vec<(u32, TbdSatTile, Decoded)> = Vec::with_capacity(plan.len());
    for ((rel, tile), bytes) in plan.into_iter().zip(bodies) {
        let Some(d) = decode_webp(&bytes, webgl2).await else {
            browser_platform::console_error!(
                "satellite: WebP decode failed for the {}x{} tile at ({},{}) of relative level \
                 {rel} ({} B, webgl2={webgl2})",
                tile.width,
                tile.height,
                tile.x,
                tile.y,
                bytes.len()
            );
            return false;
        };
        levels.push((rel, tile, d));
    }

    {
        let mut guard = engine.borrow_mut();
        let Some(e) = guard.sink_mut() else {
            browser_platform::console_error!(
                "satellite: the render engine went away before upload"
            );
            return false;
        };
        let layer = TextureLayerSpec {
            role: ROLE_BASEMAP,
            world_min: [0.0, 0.0],
            world_max: [terrain_w, terrain_h],
            width: base_mip.width,
            height: base_mip.height,
            mip_count,
            mode: MODE_UNIFIED,
        };
        if let Err(err) = e.tex_layer_begin(&layer) {
            browser_platform::console_error!(
                "satellite: could not allocate the {}x{} basemap texture with {mip_count} mips: \
                 {err:?}",
                base_mip.width,
                base_mip.height
            );
            return false;
        }
        for (rel, tile, d) in levels {
            if !upload_decoded(e, ROLE_BASEMAP, rel, tile.x, tile.y, d) {
                browser_platform::console_error!(
                    "satellite: GPU upload rejected the {}x{} tile at ({},{}) of relative level \
                     {rel}",
                    tile.width,
                    tile.height,
                    tile.x,
                    tile.y
                );
                return false;
            }
        }
        if let Err(err) = e.tex_layer_commit(ROLE_BASEMAP, 1.0_f32, true) {
            browser_platform::console_error!("satellite: basemap commit failed: {err:?}");
            return false;
        }
    }

    browser_platform::console_log!(
        "satellite: basemap up — level {base}, {}x{} with {mip_count} mips ({total} B over {} \
         Range requests); GPU maxTextureDimension2D = {}",
        base_mip.width,
        base_mip.height,
        req_count,
        limit.map_or(0, |l| l.device)
    );
    {
        let mut b = bridge.borrow_mut();
        b.sat_w = base_mip.width;
        b.sat_h = base_mip.height;
        b.sat_mode = "unified".into();
        b.sat_mips = mip_count;
    }
    publish(bridge);
    true
}
