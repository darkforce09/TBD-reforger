//! Role: glyph math.
//! Position: `overlay/symbology/labels` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::culling::lod::REF_ZOOM;

/// Readability floor (px): never shrink a glyph below this on screen.
pub const GLYPH_SIZE_MIN_PX: f64 = 4.0;

/// Building badge min pixels (`buildingLayer.ts`).
pub const BADGE_SIZE_MIN_PX: f64 = 8.0;

/// building-badge-* baseSizePx.
pub const BADGE_BASE_SIZE_PX: f64 = 10.0;

/// Reference tree height (m) at which the size multiplier is 1.0.
pub const REF_TREE_HEIGHT_M: f64 = 10.0;

/// Fallback when a prefab omits `render.baseSizePx`.
pub const DEFAULT_BASE_SIZE_PX: f64 = 16.0;

/// Fallback glyph tint (neutral forest green).
pub const DEFAULT_GLYPH_RGBA: [u8; 4] = [74, 122, 50, 255];

/// Packed icon instance stride (pos2 + size + yaw_i16 + glyph_u16 + tint_u32).
pub const ICON_INSTANCE_STRIDE: usize = 20;

/// Export yaw (clockwise from north) → Deck/screen CCW degrees. Never returns −0.
#[must_use]
pub fn deck_angle_for_rotation_deg(rotation_deg: f64) -> f64 {
    if !rotation_deg.is_finite() {
        return 0.0;
    }
    if rotation_deg == 0.0 {
        0.0
    } else {
        -rotation_deg
    }
}

/// Glyph size multiplier from tree height — clamped to [1.0, 1.5].
#[must_use]
pub fn tree_size_multiplier(height_m: Option<f64>) -> f64 {
    let Some(h) = height_m else {
        return 1.0;
    };
    if !h.is_finite() || h <= 0.0 {
        return 1.0;
    }
    let mult = h / REF_TREE_HEIGHT_M;

    if mult < 1.0 {
        return 1.0;
    }
    mult.clamp(1.0, 1.5)
}

/// Glyph size in meters for `sizeUnits:'meters'`: baseSizePx·mult / 2^REF_ZOOM.
#[must_use]
pub fn glyph_size_meters(base_size_px: f64, height_m: Option<f64>) -> f64 {
    (base_size_px * tree_size_multiplier(height_m)) / 2.0_f64.powf(REF_ZOOM)
}

/// Badge size in meters (base 10 / 2^REF_ZOOM).
#[must_use]
pub fn badge_size_meters() -> f64 {
    BADGE_BASE_SIZE_PX / 2.0_f64.powf(REF_ZOOM)
}

/// Effective size with min-pixel clamp: `max(size_m, min_px · 2^−zoom)`.
#[must_use]
pub fn size_with_min_px(size_m: f64, min_px: f64, deck_zoom: f64) -> f64 {
    let floor = min_px * 2.0_f64.powf(-deck_zoom);
    if size_m > floor { size_m } else { floor }
}

/// `#rgb` / `#rrggbb` (with or without `#`) → RGBA; invalid → DEFAULT_GLYPH_RGBA.
#[must_use]
pub fn hex_to_rgba(hex: Option<&str>) -> [u8; 4] {
    let Some(raw) = hex else {
        return DEFAULT_GLYPH_RGBA;
    };
    let h = raw.trim().trim_start_matches('#');
    let expand: String = if h.len() == 3 {
        h.chars().flat_map(|c| [c, c]).collect()
    } else {
        h.to_string()
    };
    if expand.len() != 6 || !expand.chars().all(|c| c.is_ascii_hexdigit()) {
        return DEFAULT_GLYPH_RGBA;
    }
    let r = u8::from_str_radix(&expand[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&expand[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&expand[4..6], 16).unwrap_or(0);
    [r, g, b, 255]
}

/// Pack RGBA8 as little-endian `u32` (r | g<<8 | b<<16 | a<<24).
#[must_use]
pub fn pack_rgba_u32(rgba: [u8; 4]) -> u32 {
    u32::from(rgba[0])
        | (u32::from(rgba[1]) << 8)
        | (u32::from(rgba[2]) << 16)
        | (u32::from(rgba[3]) << 24)
}

fn wrap_deg_180(angle_deg: f64) -> f64 {
    let w = angle_deg % 360.0;
    if w > 180.0 {
        w - 360.0
    } else if w <= -180.0 {
        w + 360.0
    } else {
        w
    }
}

/// Encode screen-CCW degrees as the lane's `snorm16` (`angle/180` × 32767, the angle first wrapped into `(-180, 180]` by `wrap_deg_180` — see there for why this is NOT a clamp).
#[must_use]
pub fn yaw_to_snorm16(angle_deg: f64) -> i16 {
    if !angle_deg.is_finite() || angle_deg == 0.0 {
        return 0;
    }
    let n = wrap_deg_180(angle_deg) / 180.0;
    #[allow(clippy::cast_possible_truncation)]
    {
        (n * 32767.0).round() as i16
    }
}

/// Pack one 20 B icon instance (WORLD coords for pos) into `out`.
pub fn pack_icon_instance(
    out: &mut Vec<u8>,
    pos_x: f32,
    pos_y: f32,
    size_m: f32,
    yaw_deg: f64,
    glyph: u16,
    tint: u32,
) {
    out.extend_from_slice(&pos_x.to_le_bytes());
    out.extend_from_slice(&pos_y.to_le_bytes());
    out.extend_from_slice(&size_m.to_le_bytes());
    out.extend_from_slice(&yaw_to_snorm16(yaw_deg).to_le_bytes());
    out.extend_from_slice(&glyph.to_le_bytes());
    out.extend_from_slice(&tint.to_le_bytes());
}

/// Canonical building classes value.
pub const BUILDING_CLASSES: &[&str] = &[
    "residential",
    "civic",
    "agricultural",
    "industrial",
    "commercial",
    "hangar",
    "bunker",
    "tower",
    "military",
    "bridge",
    "castle",
    "lighthouse",
    "shed",
    "container",
    "tent",
    "ruin",
    "garage",
    "generic",
];

/// Center glyph icon key for a building class (`building-{class}`).
#[must_use]
pub fn building_icon_key(building_class: &str) -> Option<&'static str> {
    match building_class {
        "residential" => Some("building-residential"),
        "civic" => Some("building-civic"),
        "agricultural" => Some("building-agricultural"),
        "industrial" => Some("building-industrial"),
        "commercial" => Some("building-commercial"),
        "hangar" => Some("building-hangar"),
        "bunker" => Some("building-bunker"),
        "tower" => Some("building-tower"),
        "military" => Some("building-military"),
        "bridge" => Some("building-bridge"),
        "castle" => Some("building-castle"),
        "lighthouse" => Some("building-lighthouse"),
        "shed" => Some("building-shed"),
        "container" => Some("building-container"),
        "tent" => Some("building-tent"),
        "ruin" => Some("building-ruin"),
        "garage" => Some("building-garage"),
        "generic" => Some("building-generic"),
        _ => None,
    }
}

/// Badge overlay icon key (`building-badge-*`) for military / tower / bunker.
#[must_use]
pub fn badge_icon_key(building_class: &str) -> Option<&'static str> {
    match building_class {
        "military" => Some("building-badge-military"),
        "tower" => Some("building-badge-tower"),
        "bunker" => Some("building-badge-bunker"),
        _ => None,
    }
}

/// Landmark / badge compose key: badge overlay wins for military/tower/bunker, else footprint glyph.
#[must_use]
pub fn landmark_glyph_icon_key(building_class: &str) -> Option<&'static str> {
    badge_icon_key(building_class).or_else(|| building_icon_key(building_class))
}

#[cfg(test)]
#[path = "tests/glyph_math_tests.rs"]
mod tests;
