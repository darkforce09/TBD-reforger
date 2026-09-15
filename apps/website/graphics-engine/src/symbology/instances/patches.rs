//! Role: patches.
//! Position: `symbology/instances` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::symbology::instances::packing::pack_icon_instance;
use crate::symbology::instances::packing::pack_rgba_u32;
use crate::symbology::instances::symbols::SLOT_GLYPH_RING;
use crate::symbology::instances::symbols::SLOT_RING_PX;
use crate::symbology::instances::symbols::SLOT_SELECTED_PX;
use crate::symbology::instances::symbols::SLOT_SELECTED_RGBA;
use crate::symbology::instances::symbols::pack_slot_symbology;
use crate::symbology::roles::classify::SIDE_BLUFOR_RGBA;

/// Pack only selected slot rings (cluster short-lane / selection-only path). Full-doc row index is **not** preserved — output is dense k selected instances.
#[must_use]
pub fn pack_selection_only(xy: &[f32], selected: &[bool]) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::new();
    let tint = pack_rgba_u32(SLOT_SELECTED_RGBA);
    for i in 0..n {
        if !selected.get(i).copied().unwrap_or(false) {
            continue;
        }
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        pack_icon_instance(&mut out, x, y, SLOT_SELECTED_PX, SLOT_GLYPH_RING, tint);
    }
    out
}

/// 12 B hide patch for base-lane size/yaw/glyph/tint at instance offset+8 (alpha 0 tint).
#[must_use]
pub fn hide_slot_row_patch() -> [u8; 12] {
    let mut hide = [0u8; 12];
    hide[0..4].copy_from_slice(&SLOT_SELECTED_PX.to_le_bytes());

    hide
}

/// Selected row patch.
#[must_use]
pub fn selected_row_patch() -> [u8; 12] {
    let mut p = [0u8; 12];
    p[0..4].copy_from_slice(&SLOT_SELECTED_PX.to_le_bytes());

    p[8..12].copy_from_slice(&pack_rgba_u32(SLOT_SELECTED_RGBA).to_le_bytes());
    p
}

/// Unselected row patch for.
#[must_use]
pub fn unselected_row_patch_for(rgba: [u8; 4]) -> [u8; 12] {
    let mut p = [0u8; 12];
    p[0..4].copy_from_slice(&SLOT_RING_PX.to_le_bytes());
    p[8..12].copy_from_slice(&pack_rgba_u32(rgba).to_le_bytes());
    p
}

/// Unselected patch with BLUFOR tint (compat / default when side unknown).
#[must_use]
pub fn unselected_row_patch() -> [u8; 12] {
    unselected_row_patch_for(SIDE_BLUFOR_RGBA)
}

/// Build a dense `selected[i]` mask from SoA ids + selected id set.
#[must_use]
pub fn selected_mask(ids: &[String], selected: &std::collections::HashSet<String>) -> Vec<bool> {
    ids.iter().map(|id| selected.contains(id)).collect()
}

/// Symbology row patch.
#[must_use]
pub fn symbology_row_patch(
    selected: bool,
    role: &str,
    heading_deg: f32,
    side_rgba: [u8; 4],
    m_per_px: f32,
    glyph_base: u16,
) -> [u8; 12] {
    let row = pack_slot_symbology(
        &[0.0, 0.0],
        &[selected],
        &[side_rgba],
        std::slice::from_ref(&role.to_string()),
        &[heading_deg],
        m_per_px,
        glyph_base,
    );
    let mut p = [0u8; 12];
    p.copy_from_slice(&row[8..20]);
    p
}
