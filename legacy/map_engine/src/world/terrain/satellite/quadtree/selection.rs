//! Role: selection.
//! Position: `world/terrain/satellite/quadtree` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::EngineHandle;
use super::TbdSatIndex;
use super::fetch_range_resilient;
use super::parse_tbd_sat_index_only;
use super::parse_tbd_sat_index_strict;

/// Canonical role basemap value.
pub(super) const ROLE_BASEMAP: u32 = 0;

/// Canonical mode unified value.
pub(super) const MODE_UNIFIED: u32 = 0;

/// Canonical mode single value.
pub(super) const MODE_SINGLE: u32 = 2;

/// Texture limit.
#[derive(Clone, Copy)]
pub(super) struct TextureLimit {
    /// Device.
    pub(super) device: u32,

    /// Adapter.
    pub(super) adapter: u32,
}

/// Texture limit.
pub(super) fn texture_limit(engine: &EngineHandle) -> Option<TextureLimit> {
    let guard = engine.borrow();
    let e = guard.as_ref()?;
    Some(TextureLimit {
        device: e.max_texture_dimension_2d(),
        adapter: e.adapter_max_texture_dimension_2d(),
    })
}

/// Report chosen level.
pub(super) fn report_chosen_level(index: &TbdSatIndex, base: usize, limit: TextureLimit) {
    let Some(mip) = index.mips.get(base) else {
        return;
    };
    if base == 0 {
        return;
    }
    let cause = if limit.device >= limit.adapter {
        "this GPU cannot hold the full-resolution basemap as a single texture"
    } else {
        "the device was granted less than the adapter offered — this is a renderer bug, not a GPU \
         limit"
    };

    let raised = crate::streaming::memory::budget::with_ledger(|l| l.satellite_raised());
    let budget_note = if raised > 0 {
        format!(
            " — and {raised} of those level(s) were taken by the MEMORY BUDGET, not by the GPU: \
             see the `memory budget` warnings above"
        )
    } else {
        String::new()
    };
    crate::diagnostics::platform::console::warn!(
        "satellite: DOWNSCALED basemap — showing level {} ({}x{}) instead of level 0 ({}x{}). GPU \
         maxTextureDimension2D = {} (adapter {}); {}{}.",
        base,
        mip.width,
        mip.height,
        index.base_width_px,
        index.base_height_px,
        limit.device,
        limit.adapter,
        cause,
        budget_note
    );
}

/// Fetch index head.
pub(super) async fn fetch_index_head(url: &str, strict: bool) -> Option<(TbdSatIndex, u64)> {
    let head = fetch_range_resilient(url, 0, 11).await?;
    let end = crate::world::terrain::satellite::streamer::index_range_end(&head.bytes).ok()?;
    let full = fetch_range_resilient(url, 0, end).await?;
    let index = if strict {
        parse_tbd_sat_index_strict(&full.bytes, full.total).ok()?
    } else {
        parse_tbd_sat_index_only(&full.bytes, full.total).ok()?
    };
    Some((index, full.total))
}
