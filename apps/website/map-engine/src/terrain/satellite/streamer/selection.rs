//! Role: selection.
//! Position: `terrain/satellite/streamer` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::TbdSatIndex;
use super::TbdSatMip;

/// First mip whose long edge fits `max_texture_dimension_2d`.
pub fn pick_base_level(index: &TbdSatIndex, max_texture_dimension_2d: u32) -> u32 {
    for mip in &index.mips {
        if mip.width.max(mip.height) <= max_texture_dimension_2d {
            return mip.level;
        }
    }
    index.mip_count.saturating_sub(1)
}

/// Pick base level for limit.
#[must_use]
pub fn pick_base_level_for_limit(
    index: &TbdSatIndex,
    max_texture_dimension_2d: Option<u32>,
) -> Option<u32> {
    max_texture_dimension_2d.map(|max| pick_base_level(index, max))
}

/// Coarsest-usable preview mip (long edge ≤ `max_edge_px`).
pub fn pick_preview_level(index: &TbdSatIndex, max_edge_px: u32) -> &TbdSatMip {
    for mip in &index.mips {
        if mip.width.max(mip.height) <= max_edge_px {
            return mip;
        }
    }
    index.mips.last().unwrap()
}
