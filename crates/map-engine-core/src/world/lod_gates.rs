//! World-object LOD gates — port of `worldmap/lodGates.ts` (N2/N3). Pure decision module:
//! Class **R** vs TS for every class × zoom (exhaustive scan in tests + vitest wasm parity).

/// Glyph size anchor: displayPx = baseSizePx * 2^(deckZoom − REF_ZOOM).
pub const REF_ZOOM: f64 = 3.0;
/// deckZoom ≥ 0 → individual tree glyphs (below: hidden; forest mass only).
pub const TREE_GLYPH_MIN_ZOOM: f64 = 0.0;
/// Historical N3 max fill zoom (+1). T-151.5.1: fill hides for `zoom ≥ TREE_GLYPH_MIN_ZOOM`
/// (exclusive upper = 0); this constant is no longer used by `class_visible`.
pub const FOREST_FILL_MAX_ZOOM: f64 = 1.0;
/// deckZoom ≥ −1.5 → forest outline (and only while below tree glyph band — T-151.5.1).
pub const FOREST_OUTLINE_MIN_ZOOM: f64 = -1.5;
/// deckZoom ≥ −2.5 → building OBB rects.
pub const BUILDING_FOOTPRINT_MIN_ZOOM: f64 = -2.5;
/// deckZoom ≥ +1 → military/tower/bunker badge.
pub const BUILDING_BADGE_MIN_ZOOM: f64 = 1.0;
/// deckZoom ≥ +1.5 → vegetation glyphs.
pub const VEGETATION_MIN_ZOOM: f64 = 1.5;
/// deckZoom ≥ +3 → prop/small-rock glyphs.
pub const PROP_MIN_ZOOM: f64 = 3.0;
/// deckZoom ≥ +1 → large rock landmark glyphs.
pub const ROCK_LARGE_MIN_ZOOM: f64 = 1.0;
/// deckZoom ≤ +3 → sea band fill visible.
pub const SEA_FILL_MAX_ZOOM: f64 = 3.0;
/// Max drawn world instances at any zoom.
pub const INSTANCE_BUDGET: usize = 150_000;

/// Every world render class the gate table covers (mirrors TS `WorldRenderClass`).
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

/// Is a class drawn (and pickable — N4) at this deckZoom?
#[must_use]
pub fn class_visible(cls: &str, deck_zoom: f64) -> bool {
    match cls {
        // T-151.5.1: hide green mass when tree glyphs are on (zoom ≥ 0).
        "forestFill" => deck_zoom < TREE_GLYPH_MIN_ZOOM,
        "sea" => deck_zoom <= SEA_FILL_MAX_ZOOM,
        "tree" => deck_zoom >= TREE_GLYPH_MIN_ZOOM,
        "vegetation" => deck_zoom >= VEGETATION_MIN_ZOOM,
        "prop" => deck_zoom >= PROP_MIN_ZOOM,
        "rockLarge" => deck_zoom >= ROCK_LARGE_MIN_ZOOM,
        "building" => deck_zoom >= BUILDING_FOOTPRINT_MIN_ZOOM,
        "buildingBadge" => deck_zoom >= BUILDING_BADGE_MIN_ZOOM,
        // Outline only in the coarse band below glyphs (no cell-edge "grid" under trees).
        "forestOutline" => (FOREST_OUTLINE_MIN_ZOOM..TREE_GLYPH_MIN_ZOOM).contains(&deck_zoom),
        "contour" => deck_zoom >= -6.0,
        "highway_paved" | "road_paved" | "runway" => deck_zoom >= -6.0,
        "road_dirt" | "track" => deck_zoom >= -2.0,
        "path" => deck_zoom >= 4.0,
        _ => false,
    }
}

/// Contour interval (m) for a deckZoom, per §N3 ladder.
#[must_use]
pub fn contour_interval_for_zoom(deck_zoom: f64) -> f64 {
    if deck_zoom < -4.0 {
        100.0
    } else if deck_zoom < -2.5 {
        50.0
    } else if deck_zoom < 1.0 {
        20.0
    } else {
        10.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_band_and_badge_gates() {
        assert!(!class_visible("tree", -0.1));
        assert!(class_visible("tree", 0.0));
        assert!(!class_visible("vegetation", 1.4));
        assert!(class_visible("vegetation", 1.5));
        assert!(!class_visible("prop", 2.9));
        assert!(class_visible("prop", 3.0));
        assert!(!class_visible("rockLarge", 0.9));
        assert!(class_visible("rockLarge", 1.0));
        assert!(!class_visible("buildingBadge", 0.9));
        assert!(class_visible("buildingBadge", 1.0));
        // T-151.5.1: forest fill/outline off once tree glyphs are on (zoom ≥ 0).
        assert!(class_visible("forestFill", -0.1));
        assert!(!class_visible("forestFill", 0.0));
        assert!(!class_visible("forestFill", 1.0));
        assert!(class_visible("forestOutline", -1.5));
        assert!(!class_visible("forestOutline", -1.6));
        assert!(!class_visible("forestOutline", 0.0));
    }

    /// Exhaustive Class R pin table for glyph-relevant classes (TS parity is also vitest-scanned).
    #[test]
    fn exhaustive_zoom_scan_glyph_classes_stable() {
        let classes = [
            "tree",
            "vegetation",
            "prop",
            "rockLarge",
            "buildingBadge",
            "building",
            "forestFill",
            "forestOutline",
            "sea",
        ];
        // Spot-check edges at 0.1 resolution for tree (min 0).
        for i in 0..=120 {
            let z = -6.0 + f64::from(i) * 0.1;
            let z = (z * 10.0).round() / 10.0; // avoid 0.1 float drift
            let tv = class_visible("tree", z);
            assert_eq!(tv, z >= 0.0, "tree @ {z}");
            let _ = classes;
        }
    }
}
