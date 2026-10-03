//! Zoom gates: what is legible at this scale, and the contour interval.
//!
//! **Role:** the zoom thresholds per world render class ([`class_visible`],
//! [`building_visible`]), the instance budget, metres per pixel at a zoom
//! ([`px_to_m_at_zoom`]) and the contour interval ladder ([`contour_interval_for_zoom`]).
//! **Position:** `map_draw_lanes`; the map engine's residency, draw buffers, viewport and
//! readouts and the unit symbology's caption sizing read it.
//! **Signals & state:** none; constants and pure functions of the zoom.
//! **Invariants:** these are cartographic legibility rules, not culling: nothing here asks
//! whether a thing is on screen, only whether drawing it at this scale helps. A zoom is the
//! log2 of pixels per metre, so one CSS pixel spans `2^-zoom` metres.

/// deckZoom ≥ 0 → individual tree glyphs (below: hidden; forest mass only).
pub const TREE_GLYPH_MIN_ZOOM: f64 = 0.0;

/// Canonical forest fill max zoom value.
pub const FOREST_FILL_MAX_ZOOM: f64 = 1.0;

/// Canonical forest outline min zoom value.
pub const FOREST_OUTLINE_MIN_ZOOM: f64 = -1.5;

/// deckZoom ≥ −2.5 → building OBB rects.
pub const BUILDING_FOOTPRINT_MIN_ZOOM: f64 = -2.5;

/// deckZoom ≥ +1 → military/tower/bunker badge.
pub const BUILDING_BADGE_MIN_ZOOM: f64 = 1.0;

/// deckZoom ≥ +1.5 → vegetation glyphs.
pub const VEGETATION_MIN_ZOOM: f64 = 1.5;

/// deckZoom ≥ +3 → prop/small-rock glyphs.
pub const PROP_MIN_ZOOM: f64 = 3.0;

/// Canonical fence min zoom value.
pub const FENCE_MIN_ZOOM: f64 = 1.5;

/// Canonical pier min zoom value.
pub const PIER_MIN_ZOOM: f64 = -1.0;

/// deckZoom ≥ +1 → large rock landmark glyphs.
pub const ROCK_LARGE_MIN_ZOOM: f64 = 1.0;

/// deckZoom ≤ +3 → sea band fill visible.
pub const SEA_FILL_MAX_ZOOM: f64 = 3.0;

/// Max drawn world instances at any zoom.
pub const INSTANCE_BUDGET: usize = 150_000;

/// Every world render class the gate table covers.
pub const WORLD_RENDER_CLASSES: &[&str] = &[
    "tree",
    "vegetation",
    "prop",
    "rockLarge",
    "building",
    "buildingBadge",
    "forestFill",
    "forestOutline",
    "sea",
    "contour",
    "highway_paved",
    "road_paved",
    "road_dirt",
    "track",
    "path",
    "runway",
];

/// Is a class drawn (and pickable) at this deckZoom?
#[must_use]
pub fn class_visible(cls: &str, deck_zoom: f64) -> bool {
    match cls {
        "forestFill" => deck_zoom < TREE_GLYPH_MIN_ZOOM,
        "sea" => deck_zoom <= SEA_FILL_MAX_ZOOM,
        "tree" => deck_zoom >= TREE_GLYPH_MIN_ZOOM,
        "vegetation" => deck_zoom >= VEGETATION_MIN_ZOOM,
        "prop" => deck_zoom >= PROP_MIN_ZOOM,

        "fence" => deck_zoom >= FENCE_MIN_ZOOM,
        "pier" => deck_zoom >= PIER_MIN_ZOOM,
        "rockLarge" => deck_zoom >= ROCK_LARGE_MIN_ZOOM,
        "building" => deck_zoom >= BUILDING_FOOTPRINT_MIN_ZOOM,
        "buildingBadge" => deck_zoom >= BUILDING_BADGE_MIN_ZOOM,

        "forestOutline" => (FOREST_OUTLINE_MIN_ZOOM..TREE_GLYPH_MIN_ZOOM).contains(&deck_zoom),
        "contour" => deck_zoom >= -6.0,
        "highway_paved" | "road_paved" | "runway" => deck_zoom >= -6.0,
        "road_dirt" | "track" => deck_zoom >= -2.0,
        "path" => deck_zoom >= 4.0,
        _ => false,
    }
}

/// Are building footprints (the oriented fill and outline rectangles) drawn at this deckZoom?
/// The `"building"` gate of [`class_visible`], named for the residency's footprint, viewport and
/// toggle callers.
#[inline]
#[must_use]
pub fn building_visible(deck_zoom: f64) -> bool {
    deck_zoom >= BUILDING_FOOTPRINT_MIN_ZOOM
}

/// Meters per CSS pixel at deck zoom (`scale = 2^zoom` → m/px = `2^(-zoom)`); a non-finite zoom
/// reads as one metre per pixel.
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

/// Band centre — the geometric mean `√(14·19)` of the 14–19 px acceptance band — the on-screen spacing eqn (2) targets. (Band edges live in the acceptance test, which owns the ±px oracle.).
pub const TARGET_SPACING_PX: f64 = 16.309_506_430_300_09;

/// Representative Everon interior gradient (rise/run) the contours cross — `tan(11°)`. Median of the DEM slope statistics; converts an elevation interval to a horizontal on-ground distance in (1)/(2).
pub const CONTOUR_REPRESENTATIVE_SLOPE: f64 = 0.194_380_309_147_231_4;

/// Finest ground interval (m) — the high-zoom rung.
pub const CONTOUR_INTERVAL_MIN_M: f64 = 5.0;

/// Coarsest ground interval (m) — the whole-terrain rung (deckZoom ≥ −6 ⇒ m/pix ≤ 64).
pub const CONTOUR_INTERVAL_MAX_M: f64 = 80.0;

/// A ×2 ladder lands spacing IN the band (14–19 px) at each rung's centre m/pix — which is where the corpus shows Eden flipping the interval — and within ≈[11.5, 23] px at the rung boundaries (the tightest a doubling ladder can hold; a finer band would need a non-doubling interval set).
#[must_use]
pub fn contour_interval_for_zoom(m_per_px: f64) -> f64 {
    if !m_per_px.is_finite() || m_per_px <= 0.0 {
        return CONTOUR_INTERVAL_MIN_M;
    }

    let ideal = TARGET_SPACING_PX * CONTOUR_REPRESENTATIVE_SLOPE * m_per_px;

    let steps = (ideal / CONTOUR_INTERVAL_MIN_M).log2().round();
    (CONTOUR_INTERVAL_MIN_M * 2.0_f64.powf(steps))
        .clamp(CONTOUR_INTERVAL_MIN_M, CONTOUR_INTERVAL_MAX_M)
}

#[cfg(test)]
#[path = "tests/zoom_gates_tests.rs"]
mod tests;
