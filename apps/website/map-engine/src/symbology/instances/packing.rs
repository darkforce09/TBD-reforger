//! Role: packing.
//! Position: `symbology/instances` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Pack RGBA8 as little-endian `u32` (r | g<<8 | b<<16 | a<<24).
#[must_use]
pub fn pack_rgba_u32(rgba: [u8; 4]) -> u32 {
    u32::from(rgba[0])
        | (u32::from(rgba[1]) << 8)
        | (u32::from(rgba[2]) << 16)
        | (u32::from(rgba[3]) << 24)
}

/// Pack one 20 B icon instance (WORLD meters for pos; size in **pixels** for slot atlas with `px_to_m` uniform, or meters when `px_to_m = 1`).
pub fn pack_icon_instance(
    out: &mut Vec<u8>,
    pos_x: f32,
    pos_y: f32,
    size_px: f32,
    glyph: u16,
    tint: u32,
) {
    pack_icon_instance_yaw(out, pos_x, pos_y, size_px, 0.0, glyph, tint);
}

/// Pack icon instance yaw.
pub fn pack_icon_instance_yaw(
    out: &mut Vec<u8>,
    pos_x: f32,
    pos_y: f32,
    size_px: f32,
    yaw_deg: f64,
    glyph: u16,
    tint: u32,
) {
    out.extend_from_slice(&pos_x.to_le_bytes());
    out.extend_from_slice(&pos_y.to_le_bytes());
    out.extend_from_slice(&size_px.to_le_bytes());
    out.extend_from_slice(&yaw_to_snorm16(yaw_deg).to_le_bytes());
    out.extend_from_slice(&glyph.to_le_bytes());
    out.extend_from_slice(&tint.to_le_bytes());
}

/// Wrap deg 180.
pub(crate) fn wrap_deg_180(angle_deg: f64) -> f64 {
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

/// Document heading → the screen-CCW yaw the icon shader wants.
#[must_use]
pub fn screen_yaw_for_heading_deg(heading_deg: f64) -> f64 {
    if !heading_deg.is_finite() || heading_deg == 0.0 {
        return 0.0;
    }
    -heading_deg
}
