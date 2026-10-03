//! Town label importance: which named locations draw at a zoom, and how they fade.
//!
//! **Role:** holds the [`LocationLabel`] rows of a terrain's `locations.json`, sizes a
//! location's land footprint from its importance, and decides per zoom which towns draw
//! ([`should_draw_town_label`], [`declutter_town_labels`]) and with which fade alpha.
//! **Position:** `label_layout`; the map engine's location loader and town packers read it.
//! **Signals & state:** none; constants and pure functions.
//! **Invariants:** a town draws only inside the zoom band, with a kind the zoom admits, and
//! when no more important location stands within its importance-scaled threshold; the
//! threshold doubles with every zoom step out.

use crate::label_ids::LocationId;

/// Canonical importance scale value.
pub const IMPORTANCE_SCALE: f64 = 0.08;

/// Base land footprint meters at importance = 1.
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

/// `true` ⇒ fade over `[MAX_ZOOM, FADE_END]`; `false` ⇒ hard hide at `MAX_ZOOM`.
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

/// Whether a location `kind` belongs on the town label lane at `deck_zoom`: `town` (also when
/// absent), `village` and `airport` always, `locality` only at zoom 0 and closer, others never.
#[must_use]
pub fn town_lane_kind_ok(kind: Option<&str>, deck_zoom: f64) -> bool {
    match kind.unwrap_or("town") {
        "town" | "village" | "airport" => true,
        "locality" => deck_zoom >= 0.0,
        _ => false,
    }
}

/// Town label opacity in `[0, 1]` at `deck_zoom`: 1 up to [`TOWN_LABEL_MAX_ZOOM`], falling
/// linearly to 0 at [`TOWN_LABEL_FADE_END`]; a hard 1 or 0 step when fading is disabled.
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
    /// The location's id, such as `everon-airport`.
    pub id: LocationId,

    /// Display text of the label (JSON `name`); a trimmed name under 2 bytes never draws.
    pub name: String,

    /// East coordinate in world metres (JSON `x`).
    pub x: f64,

    /// North coordinate in world metres, the world Z axis (JSON `y`).
    pub y: f64,

    /// Declutter importance from 0 (minor) to 1 (capital) (JSON `importance`, default 0.5).
    #[serde(default = "default_importance")]
    pub importance: f64,

    /// Location taxonomy such as `town`, `village`, `locality`, `airport`, `peak` (JSON `kind`);
    /// absent reads as `town`.
    #[serde(default)]
    pub kind: Option<String>,
}

fn default_importance() -> f64 {
    0.5
}

/// Land footprint in metres of a location: `sqrt(importance)` × [`TOWN_BASE_SIZE_M`], with
/// importance clamped to `[0, 1]`.
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

/// Whether `loc` draws at `deck_zoom`: inside the town zoom band, a kind the zoom admits,
/// important enough when zoomed wide, a name of 2 bytes or more, and no more important
/// location within its declutter threshold.
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

/// Every drawn label satisfies the per-label draw predicate [`should_draw_town_label`].
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
