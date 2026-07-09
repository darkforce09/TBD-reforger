//! Glyph size / angle / color pure math — Class **R** port of `treePropLayer.ts`.

use super::lod_gates::REF_ZOOM;

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
    // Match TS: shorter trees clamp up to 1.0 (never shrink undergrowth glyphs).
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

/// Encode screen CCW degrees as snorm16 (`angle/180` clamped to [−1,1] × 32767).
#[must_use]
pub fn yaw_to_snorm16(angle_deg: f64) -> i16 {
    if !angle_deg.is_finite() || angle_deg == 0.0 {
        return 0;
    }
    let n = (angle_deg / 180.0).clamp(-1.0, 1.0);
    (n * 32767.0).round() as i16
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

/// Badge icon key for a building class (`military` / `tower` / `bunker` only).
#[must_use]
pub fn badge_icon_key(building_class: &str) -> Option<&'static str> {
    match building_class {
        "military" => Some("building-badge-military"),
        "tower" => Some("building-badge-tower"),
        "bunker" => Some("building-badge-bunker"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_angle_handedness() {
        assert_eq!(deck_angle_for_rotation_deg(0.0), 0.0);
        assert_eq!(deck_angle_for_rotation_deg(90.0), -90.0);
        assert_eq!(deck_angle_for_rotation_deg(180.0), -180.0);
        assert!(!deck_angle_for_rotation_deg(0.0).is_sign_negative());
        assert_eq!(deck_angle_for_rotation_deg(f64::NAN), 0.0);
    }

    #[test]
    fn tree_size_multiplier_clamps() {
        assert!((tree_size_multiplier(None) - 1.0).abs() < 1e-12);
        assert!((tree_size_multiplier(Some(10.0)) - 1.0).abs() < 1e-12);
        assert!((tree_size_multiplier(Some(5.0)) - 1.0).abs() < 1e-12);
        assert!((tree_size_multiplier(Some(12.5)) - 1.25).abs() < 1e-12);
        assert!((tree_size_multiplier(Some(20.0)) - 1.5).abs() < 1e-12);
        assert!((tree_size_multiplier(Some(100.0)) - 1.5).abs() < 1e-12);
    }

    #[test]
    fn glyph_size_meters_formula() {
        let expect = 24.0 / 2.0_f64.powf(REF_ZOOM);
        assert!((glyph_size_meters(24.0, Some(10.0)) - expect).abs() < 1e-9);
        let expect15 = (24.0 * 1.5) / 2.0_f64.powf(REF_ZOOM);
        assert!((glyph_size_meters(24.0, Some(20.0)) - expect15).abs() < 1e-9);
    }

    #[test]
    fn hex_to_rgba_parses() {
        assert_eq!(hex_to_rgba(Some("#2d5a27")), [45, 90, 39, 255]);
        assert_eq!(hex_to_rgba(Some("4a7a32")), [74, 122, 50, 255]);
        assert_eq!(hex_to_rgba(Some("#abc")), [170, 187, 204, 255]);
        assert_eq!(hex_to_rgba(None), DEFAULT_GLYPH_RGBA);
        assert_eq!(hex_to_rgba(Some("nothex")), DEFAULT_GLYPH_RGBA);
    }

    #[test]
    fn pack_icon_instance_is_20_bytes() {
        let mut v = Vec::new();
        pack_icon_instance(
            &mut v,
            1.5,
            -2.5,
            3.0,
            -90.0,
            7,
            pack_rgba_u32([45, 90, 39, 255]),
        );
        assert_eq!(v.len(), ICON_INSTANCE_STRIDE);
        assert_eq!(ICON_INSTANCE_STRIDE, 20);
        // pos x
        assert_eq!(f32::from_le_bytes(v[0..4].try_into().unwrap()), 1.5);
        assert_eq!(f32::from_le_bytes(v[4..8].try_into().unwrap()), -2.5);
        assert_eq!(f32::from_le_bytes(v[8..12].try_into().unwrap()), 3.0);
        let yaw = i16::from_le_bytes(v[12..14].try_into().unwrap());
        assert_eq!(yaw, yaw_to_snorm16(-90.0));
        assert_eq!(u16::from_le_bytes(v[14..16].try_into().unwrap()), 7);
    }

    #[test]
    fn badge_keys() {
        assert_eq!(badge_icon_key("military"), Some("building-badge-military"));
        assert_eq!(badge_icon_key("residential"), None);
    }
}
