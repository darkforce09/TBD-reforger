//! Role: preview.
//! Position: `terrain/satellite/quadtree` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::BridgeHandle;
use super::EngineHandle;
use super::MODE_SINGLE;
use super::commit_mip;
use super::fetch_index_head;
use super::fetch_mip_blocks;
use super::pick_preview_level;
use super::publish;

/// Canonical preview max edge value.
pub(super) const PREVIEW_MAX_EDGE: u32 = 1024;

/// `?sat=preview` — Range-only path; never full-bundle GET (CI / gate harness).
pub fn sat_preview_only() -> bool {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .map(|s| s.contains("sat=preview"))
        .unwrap_or(false)
}

/// Try preview.
pub(super) async fn try_preview(
    engine: &EngineHandle,
    url: &str,
    terrain_w: f64,
    terrain_h: f64,
    bridge: &BridgeHandle,
) -> bool {
    let Some((index, total)) = fetch_index_head(url, false).await else {
        return false;
    };
    let mip = pick_preview_level(&index, PREVIEW_MAX_EDGE).clone();
    let Some(blocks) = fetch_mip_blocks(url, total, &mip).await else {
        return false;
    };
    if !commit_mip(
        engine,
        terrain_w,
        terrain_h,
        &mip,
        blocks,
        MODE_SINGLE,
        1,
        1.0,
    )
    .await
    {
        return false;
    }
    {
        let mut b = bridge.borrow_mut();
        b.sat_w = mip.width;
        b.sat_h = mip.height;
        b.sat_mode = "single".into();
        b.sat_mips = 1;
    }
    publish(bridge);
    true
}
