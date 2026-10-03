//! World object glyph sizing, tinting and building icon keys.
//!
//! **Role:** sizes the world's object glyphs (trees, props, building badges) in metres from a
//! base pixel size and a tree height, clamps them to a readable on-screen floor, parses a
//! prefab's hex tint, turns an exported rotation into the screen angle the icon shader wants,
//! and names the glyph atlas key of each building class.
//! **Position:** `label_layout`; the map engine's draw buffers and glyph lookup read it, and
//! pack the bytes with `render_primitives::text::pack`.
//! **Signals & state:** none; constants and pure functions.
//! **Invariants:** a size in metres is a base pixel size measured at
//! `render_primitives::text::scale::REF_ZOOM`; an exported rotation is clockwise from north
//! and the screen angle counter-clockwise, never `-0`; every building class has a footprint
//! key, and the badge key wins for military, tower and bunker.

use render_primitives::text::scale::REF_ZOOM;

/// Readability floor (px): never shrink a glyph below this on screen.
pub const GLYPH_SIZE_MIN_PX: f64 = 4.0;

/// Readability floor (px) of a building badge.
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
