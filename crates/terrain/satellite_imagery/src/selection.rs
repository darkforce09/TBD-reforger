//! The base and preview level picks of a tile index.
//!
//! **Role:** [`pick_base_level`] and [`pick_base_level_for_limit`] return the first level whose
//! long edge fits a texture limit; [`pick_preview_level`] the first whose long edge fits a
//! preview size.
//! **Position:** reads a checked [`TbdSatIndex`]; the map engine's satellite loader chooses which
//! levels to fetch and upload with it.
//! **Signals & state:** none; pure functions.
//! **Invariants:** each pick falls back to the last, smallest level.

use crate::model::{TbdSatIndex, TbdSatMip};

/// First mip whose long edge fits `max_texture_dimension_2d`.
pub fn pick_base_level(index: &TbdSatIndex, max_texture_dimension_2d: u32) -> u32 {
    for mip in &index.mips {
        if mip.width.max(mip.height) <= max_texture_dimension_2d {
            return mip.level;
        }
    }
    index.mip_count.saturating_sub(1)
}

/// [`pick_base_level`] under an optional texture limit: `None` when no limit is known,
/// otherwise the first level whose long edge fits it (the last level when none does).
#[must_use]
pub fn pick_base_level_for_limit(
    index: &TbdSatIndex,
    max_texture_dimension_2d: Option<u32>,
) -> Option<u32> {
    max_texture_dimension_2d.map(|max| pick_base_level(index, max))
}

/// Coarsest-usable preview mip (long edge ≤ `max_edge_px`), else the last level.
///
/// # Panics
///
/// When `index` holds no level; an index that passed [`crate::parse_tbd_sat_index_only`] holds
/// at least one.
pub fn pick_preview_level(index: &TbdSatIndex, max_edge_px: u32) -> &TbdSatMip {
    for mip in &index.mips {
        if mip.width.max(mip.height) <= max_edge_px {
            return mip;
        }
    }
    index
        .mips
        .last()
        .expect("a checked tile index holds at least one level")
}
