//! Role: importance.
//! Position: `overlay/symbology/labels` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

#![forbid(unsafe_code)]

/// Canonical importance scale value.
pub const IMPORTANCE_SCALE: f64 = 0.08;

/// Base land footprint meters at importance = 1 (verify log pins).
pub const TOWN_BASE_SIZE_M: f64 = 400.0;

/// Canonical town label min zoom value.
pub const TOWN_LABEL_MIN_ZOOM: f64 = -4.5;

/// Canonical town label wide zoom value.
pub const TOWN_LABEL_WIDE_ZOOM: f64 = -3.0;

/// Canonical town label wide min importance value.
pub const TOWN_LABEL_WIDE_MIN_IMPORTANCE: f64 = 0.70;

/// Canonical town label max zoom value.
pub const TOWN_LABEL_MAX_ZOOM: f64 = 2.0;

/// Canonical town label fade end value.
pub const TOWN_LABEL_FADE_END: f64 = 3.0;

/// Operator M2: `true` ⇒ fade over `[MAX_ZOOM, FADE_END]`; `false` ⇒ hard hide at `MAX_ZOOM`.
pub const TOWN_LABEL_FADE_ENABLED: bool = true;

/// Draw ceiling: the fade end when fading, else the hard clutter cap.
#[must_use]
pub fn town_label_zoom_ceiling() -> f64 {
    if TOWN_LABEL_FADE_ENABLED {
        TOWN_LABEL_FADE_END
    } else {
        TOWN_LABEL_MAX_ZOOM
    }
}

/// Town lane kind ok.
#[must_use]
pub fn town_lane_kind_ok(kind: Option<&str>, deck_zoom: f64) -> bool {
    match kind.unwrap_or("town") {
        "town" | "village" | "airport" => true,
        "locality" => deck_zoom >= 0.0,
        _ => false,
    }
}

/// Town label fade alpha.
#[must_use]
pub fn town_label_fade_alpha(deck_zoom: f64) -> f64 {
    if !TOWN_LABEL_FADE_ENABLED {
        return f64::from(u8::from(deck_zoom <= TOWN_LABEL_MAX_ZOOM));
    }
    if deck_zoom <= TOWN_LABEL_MAX_ZOOM {
        1.0
    } else if deck_zoom >= TOWN_LABEL_FADE_END {
        0.0
    } else {
        1.0 - (deck_zoom - TOWN_LABEL_MAX_ZOOM) / (TOWN_LABEL_FADE_END - TOWN_LABEL_MAX_ZOOM)
    }
}

/// One named map location from `locations.json`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LocationLabel {
    /// Id.
    pub id: String,

    /// Name.
    pub name: String,

    /// X.
    pub x: f64,

    /// Y.
    pub y: f64,

    /// Importance.
    #[serde(default = "default_importance")]
    pub importance: f64,

    /// Kind.
    #[serde(default)]
    pub kind: Option<String>,
}

fn default_importance() -> f64 {
    0.5
}

/// Size land m.
#[must_use]
pub fn size_land_m(importance: f64) -> f64 {
    importance.clamp(0.0, 1.0).sqrt() * TOWN_BASE_SIZE_M
}

/// Declutter threshold in world meters for one label at `deck_zoom`.
#[must_use]
pub fn town_declutter_threshold_m(importance: f64, deck_zoom: f64) -> f64 {
    IMPORTANCE_SCALE * size_land_m(importance) * 2f64.powf(-deck_zoom)
}

fn dist_m(a: &LocationLabel, b: &LocationLabel) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx.hypot(dy)
}

/// Distance to the nearest strictly higher-importance neighbor (m); ∞ when none.
#[must_use]
pub fn nearest_more_important_m(loc: &LocationLabel, all: &[LocationLabel]) -> f64 {
    let imp = loc.importance;
    let mut best = f64::INFINITY;
    for other in all {
        if other.importance > imp + 1e-12 {
            let d = dist_m(loc, other);
            if d < best {
                best = d;
            }
        }
    }
    best
}

/// Should draw town label.
#[must_use]
pub fn should_draw_town_label(loc: &LocationLabel, all: &[LocationLabel], deck_zoom: f64) -> bool {
    if !(TOWN_LABEL_MIN_ZOOM..=town_label_zoom_ceiling()).contains(&deck_zoom) {
        return false;
    }

    if !town_lane_kind_ok(loc.kind.as_deref(), deck_zoom) {
        return false;
    }

    if deck_zoom < TOWN_LABEL_WIDE_ZOOM && loc.importance < TOWN_LABEL_WIDE_MIN_IMPORTANCE {
        return false;
    }
    let name = loc.name.trim();
    if name.len() < 2 {
        return false;
    }
    let nearest = nearest_more_important_m(loc, all);
    let threshold = town_declutter_threshold_m(loc.importance, deck_zoom);
    nearest >= threshold
}

/// Return the draw set at `deck_zoom`.
#[must_use]
pub fn declutter_town_labels(locations: &[LocationLabel], deck_zoom: f64) -> Vec<LocationLabel> {
    locations
        .iter()
        .filter(|l| should_draw_town_label(l, locations, deck_zoom))
        .cloned()
        .collect()
}

/// G3: every drawn label satisfies the per-label A3 predicate.
#[must_use]
pub fn town_declutter_invariant_holds(
    drawn: &[LocationLabel],
    all: &[LocationLabel],
    deck_zoom: f64,
) -> bool {
    drawn
        .iter()
        .all(|l| should_draw_town_label(l, all, deck_zoom))
}

#[cfg(test)]
#[path = "tests/importance_tests.rs"]
mod tests;
