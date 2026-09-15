//! Role: symbols.
//! Position: `symbology/instances` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::symbology::atlas::raster::COMMENT_CELL;
use crate::symbology::atlas::raster::COMMENT_SELECTED_CELL;
use crate::symbology::atlas::raster::UNIT_CELL_BASE;
use crate::symbology::atlas::raster::UNIT_SELECTED_CELL_BASE;
use crate::symbology::atlas::raster::VEHICLE_CELL_BASE;
use crate::symbology::instances::packing::pack_icon_instance;
use crate::symbology::instances::packing::pack_icon_instance_yaw;
use crate::symbology::instances::packing::pack_rgba_u32;
use crate::symbology::instances::packing::screen_yaw_for_heading_deg;
use crate::symbology::roles::classify::SIDE_BLUFOR_RGBA;
use crate::symbology::roles::classify::UnitRoleClass;
use crate::symbology::roles::classify::VehicleKind;
use crate::symbology::roles::classify::side_rgba;
use crate::symbology::roles::classify::unit_role_class;
use crate::symbology::roles::classify::vehicle_kind_for_alias;

/// Icon instance stride (pos2 + size + yaw_i16 + glyph_u16 + tint_u32).
pub const SLOT_ICON_STRIDE: usize = 20;

/// Glyph index in the dedicated slot atlas (ring).
pub const SLOT_GLYPH_RING: u16 = 0;

/// Glyph index in the dedicated slot atlas (solid disc).
pub const SLOT_GLYPH_DISC: u16 = 1;

/// Base ring size in CSS pixels (`useIconLayer` getSize).
pub const SLOT_RING_PX: f32 = 20.0;

/// Selected ring size in CSS pixels.
pub const SLOT_SELECTED_PX: f32 = 28.0;

/// Aegis primary `#adc6ff` full alpha (= BLUFOR).
pub const SLOT_PRIMARY_RGBA: [u8; 4] = SIDE_BLUFOR_RGBA;

/// Tactical yellow `#facc15` full alpha.
pub const SLOT_SELECTED_RGBA: [u8; 4] = [250, 204, 21, 255];

/// Cluster disc primary with Deck alpha 235.
pub const CLUSTER_DISC_RGBA: [u8; 4] = [173, 198, 255, 235];

/// Canonical cluster slot threshold value.
pub const CLUSTER_SLOT_THRESHOLD: u32 = 500;

/// Canonical zoom cluster max value.
pub const ZOOM_CLUSTER_MAX: f64 = -4.0;

/// Cluster mode.
#[must_use]
pub fn cluster_mode(slot_len: u32, deck_zoom: f64) -> bool {
    slot_len > CLUSTER_SLOT_THRESHOLD && deck_zoom <= ZOOM_CLUSTER_MAX
}

/// Disc pixel size from aggregated count (`useClusterIconLayer.discSize`).
#[must_use]
pub fn cluster_disc_size_px(count: u32) -> f32 {
    let c = count.max(1) as f64;
    let extra = (c.log10() * 12.0).min(26.0);
    (22.0 + extra) as f32
}

/// Pack slot rings from interleaved `xy` (`[x0,y0,…]`, length `2·n`). `selected[i]` true → yellow + 28 px, else `side_tints[i]` (or BLUFOR if short) + 20 px.
#[must_use]
pub fn pack_slot_instances(xy: &[f32], selected: &[bool], side_tints: &[[u8; 4]]) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let sel = pack_rgba_u32(SLOT_SELECTED_RGBA);
    for i in 0..n {
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        let is_sel = selected.get(i).copied().unwrap_or(false);
        let (size, tint) = if is_sel {
            (SLOT_SELECTED_PX, sel)
        } else {
            let rgba = side_tints.get(i).copied().unwrap_or(SIDE_BLUFOR_RGBA);
            (SLOT_RING_PX, pack_rgba_u32(rgba))
        };
        pack_icon_instance(&mut out, x, y, size, SLOT_GLYPH_RING, tint);
    }
    out
}

/// Pack rings.
#[must_use]
pub fn pack_rings(xy: &[f32], selected: &[bool], side_keys: &[&str]) -> Vec<u8> {
    let tints: Vec<[u8; 4]> = side_keys.iter().map(|k| side_rgba(k)).collect();
    pack_slot_instances(xy, selected, &tints)
}

/// Pack a single slot instance at world `(x,y)` with selection flag.
#[must_use]
pub fn pack_one_slot(x: f32, y: f32, selected: bool) -> [u8; SLOT_ICON_STRIDE] {
    let mut v = Vec::with_capacity(SLOT_ICON_STRIDE);
    let (size, tint) = if selected {
        (SLOT_SELECTED_PX, pack_rgba_u32(SLOT_SELECTED_RGBA))
    } else {
        (SLOT_RING_PX, pack_rgba_u32(SLOT_PRIMARY_RGBA))
    };
    pack_icon_instance(&mut v, x, y, size, SLOT_GLYPH_RING, tint);
    let mut arr = [0u8; SLOT_ICON_STRIDE];
    arr.copy_from_slice(&v);
    arr
}

/// Pack vehicle instances.
#[must_use]
pub fn pack_vehicle_instances(xy: &[f32]) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let tint = pack_rgba_u32(SLOT_SELECTED_RGBA);
    for i in 0..n {
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        pack_icon_instance(&mut out, x, y, SLOT_RING_PX, SLOT_GLYPH_DISC, tint);
    }
    out
}

/// Pack cluster disc markers: parallel `xs`/`ys`/`counts` (world meters).
#[must_use]
pub fn pack_cluster_instances(xs: &[f64], ys: &[f64], counts: &[u32]) -> Vec<u8> {
    let n = xs.len().min(ys.len()).min(counts.len());
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let tint = pack_rgba_u32(CLUSTER_DISC_RGBA);
    for i in 0..n {
        #[allow(clippy::cast_possible_truncation)]
        let x = xs[i] as f32;
        #[allow(clippy::cast_possible_truncation)]
        let y = ys[i] as f32;
        let size = cluster_disc_size_px(counts[i]);
        pack_icon_instance(&mut out, x, y, size, SLOT_GLYPH_DISC, tint);
    }
    out
}

/// Meters per CSS pixel at deck zoom (`scale = 2^zoom` → m/px = `2^(-zoom)`).
#[must_use]
pub fn px_to_m_at_zoom(deck_zoom: f64) -> f32 {
    if !deck_zoom.is_finite() {
        return 1.0;
    }
    #[allow(clippy::cast_possible_truncation)]
    {
        2.0_f64.powf(-deck_zoom) as f32
    }
}

/// Screen size (CSS px) of an unselected unit glyph. Larger than [`SLOT_RING_PX`] because the role symbol is knocked out of a 40 px-diameter body inside a 64 px cell: at 20 px the interior is ~12 px across and a 4 px-stroke knockout lands sub-pixel. Selected still uses [`SLOT_SELECTED_PX`], so the selection size step is unchanged.
pub const SLOT_UNIT_PX: f32 = 24.0;

/// Screen size (CSS px) of a vehicle silhouette — the hull is longer than it is wide, so it needs more cell than a unit disc to stay readable.
pub const VEHICLE_SYMBOL_PX: f32 = 26.0;

/// Screen size (CSS px) of an unselected comment bubble.
pub const COMMENT_NOTE_PX: f32 = 22.0;

/// Canonical comment note rgba value.
pub const COMMENT_NOTE_RGBA: [u8; 4] = [203, 213, 225, 255];

/// Canonical symbology max m per px value.
pub const SYMBOLOGY_MAX_M_PER_PX: f32 = 8.0;

/// True when the camera is close enough to draw symbology rather than dots (see [`SYMBOLOGY_MAX_M_PER_PX`]). Non-finite / non-positive scales degrade (fail safe).
#[must_use]
pub fn symbology_visible(m_per_px: f32) -> bool {
    m_per_px.is_finite() && m_per_px > 0.0 && m_per_px <= SYMBOLOGY_MAX_M_PER_PX
}

/// Pack the slot lane as UNIT SYMBOLOGY: side colour + role glyph + real heading, selection on top.
#[must_use]
pub fn pack_slot_symbology(
    xy: &[f32],
    selected: &[bool],
    side_tints: &[[u8; 4]],
    roles: &[String],
    headings_deg: &[f32],
    m_per_px: f32,
    glyph_base: u16,
) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let detailed = symbology_visible(m_per_px);
    let sel_tint = pack_rgba_u32(SLOT_SELECTED_RGBA);
    for i in 0..n {
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        let is_sel = selected.get(i).copied().unwrap_or(false);
        let tint = if is_sel {
            sel_tint
        } else {
            pack_rgba_u32(side_tints.get(i).copied().unwrap_or(SIDE_BLUFOR_RGBA))
        };
        if !detailed {
            let size = if is_sel {
                SLOT_SELECTED_PX
            } else {
                SLOT_RING_PX
            };
            pack_icon_instance(&mut out, x, y, size, SLOT_GLYPH_DISC, tint);
            continue;
        }
        let class = roles
            .get(i)
            .map_or(UnitRoleClass::Rifleman, |r| unit_role_class(r)) as u16;
        let block = if is_sel {
            UNIT_SELECTED_CELL_BASE
        } else {
            UNIT_CELL_BASE
        };
        let size = if is_sel {
            SLOT_SELECTED_PX
        } else {
            SLOT_UNIT_PX
        };
        let yaw =
            screen_yaw_for_heading_deg(f64::from(headings_deg.get(i).copied().unwrap_or(0.0)));
        pack_icon_instance_yaw(&mut out, x, y, size, yaw, glyph_base + block + class, tint);
    }
    out
}

/// Pack the mission-vehicle lane as TOP-DOWN SILHOUETTES with real heading and side colour.
#[must_use]
pub fn pack_vehicle_symbology(
    xy: &[f32],
    aliases: &[String],
    side_tints: &[[u8; 4]],
    headings_deg: &[f32],
    m_per_px: f32,
    glyph_base: u16,
) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let detailed = symbology_visible(m_per_px);
    for i in 0..n {
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        let tint = pack_rgba_u32(side_tints.get(i).copied().unwrap_or(SIDE_BLUFOR_RGBA));
        if !detailed {
            pack_icon_instance(&mut out, x, y, SLOT_RING_PX, SLOT_GLYPH_DISC, tint);
            continue;
        }
        let kind = aliases
            .get(i)
            .map_or(VehicleKind::WheeledLight, |a| vehicle_kind_for_alias(a))
            as u16;
        let yaw =
            screen_yaw_for_heading_deg(f64::from(headings_deg.get(i).copied().unwrap_or(0.0)));
        pack_icon_instance_yaw(
            &mut out,
            x,
            y,
            VEHICLE_SYMBOL_PX,
            yaw,
            glyph_base + VEHICLE_CELL_BASE + kind,
            tint,
        );
    }
    out
}

/// Replaces the previous "selection-amber ring" rendering, which borrowed both the colour and the glyph of a selected slot: a comment looked selected at all times and looked like a unit. Selected comments take [`SLOT_SELECTED_RGBA`] + [`SLOT_SELECTED_PX`] + the ringed bubble cell, so the selection treatment is additive rather than the comment's only appearance.
#[must_use]
pub fn pack_comment_instances(
    xy: &[f32],
    selected: &[bool],
    m_per_px: f32,
    glyph_base: u16,
) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let detailed = symbology_visible(m_per_px);
    for i in 0..n {
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        let is_sel = selected.get(i).copied().unwrap_or(false);
        let tint = pack_rgba_u32(if is_sel {
            SLOT_SELECTED_RGBA
        } else {
            COMMENT_NOTE_RGBA
        });
        let size = if is_sel {
            SLOT_SELECTED_PX
        } else {
            COMMENT_NOTE_PX
        };
        let glyph = if detailed {
            glyph_base
                + if is_sel {
                    COMMENT_SELECTED_CELL
                } else {
                    COMMENT_CELL
                }
        } else {
            SLOT_GLYPH_DISC
        };
        pack_icon_instance(&mut out, x, y, size, glyph, tint);
    }
    out
}
