//! A3-style importance-distance town label declutter (T-152.8).
//! Predicate: draw iff `nearest_more_important_m ≥ IMPORTANCE_SCALE · size_land_m · 2^(−z)`.

#![forbid(unsafe_code)]

/// A3 `uiMap.cpp` scale factor (T-152.8 L3).
pub const IMPORTANCE_SCALE: f64 = 0.08;
/// Base land footprint meters at importance = 1 (verify log pins).
pub const TOWN_BASE_SIZE_M: f64 = 400.0;
/// Labels hidden when zoomed out past this deck zoom.
pub const TOWN_LABEL_MIN_ZOOM: f64 = -3.0;
/// Labels hidden when zoomed in past this deck zoom (optional clutter cap).
pub const TOWN_LABEL_MAX_ZOOM: f64 = 2.0;

/// One named map location from `locations.json`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LocationLabel {
    pub id: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    #[serde(default = "default_importance")]
    pub importance: f64,
    #[serde(default)]
    pub kind: Option<String>,
}

fn default_importance() -> f64 {
    0.5
}

/// `size_land_m = √importance · TOWN_BASE_SIZE_M` (T-152.8 L3).
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

/// Whether `loc` passes the A3 predicate at `deck_zoom` (zoom band + declutter).
#[must_use]
pub fn should_draw_town_label(loc: &LocationLabel, all: &[LocationLabel], deck_zoom: f64) -> bool {
    if deck_zoom < TOWN_LABEL_MIN_ZOOM || deck_zoom > TOWN_LABEL_MAX_ZOOM {
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
pub fn town_declutter_invariant_holds(drawn: &[LocationLabel], all: &[LocationLabel], deck_zoom: f64) -> bool {
    drawn.iter().all(|l| should_draw_town_label(l, all, deck_zoom))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(id: &str, name: &str, x: f64, y: f64, imp: f64) -> LocationLabel {
        LocationLabel {
            id: id.into(),
            name: name.into(),
            x,
            y,
            importance: imp,
            kind: None,
        }
    }

    #[test]
    fn threshold_scales_with_zoom() {
        let imp = 0.7;
        let t0 = town_declutter_threshold_m(imp, 0.0);
        let tm2 = town_declutter_threshold_m(imp, -2.0);
        assert!((t0 - 0.08 * size_land_m(imp)).abs() < 1e-6);
        assert!((tm2 / t0 - 4.0).abs() < 1e-6);
    }

    #[test]
    fn close_lower_importance_dropped() {
        let capital = loc("a", "Capital", 0.0, 0.0, 0.9);
        let hamlet = loc("b", "Hamlet", 10.0, 0.0, 0.3);
        let all = vec![capital.clone(), hamlet.clone()];
        let drawn = declutter_town_labels(&all, 0.0);
        assert!(drawn.iter().any(|l| l.id == "a"));
        assert!(!drawn.iter().any(|l| l.id == "b"));
        assert!(town_declutter_invariant_holds(&drawn, &all, 0.0));
    }

    #[test]
    fn far_apart_both_kept() {
        let a = loc("a", "Alpha", 0.0, 0.0, 0.5);
        let b = loc("b", "Beta", 5000.0, 5000.0, 0.5);
        let all = vec![a, b];
        let drawn = declutter_town_labels(&all, -2.0);
        assert_eq!(drawn.len(), 2);
        assert!(town_declutter_invariant_holds(&drawn, &all, -2.0));
    }

    #[test]
    fn zoom_band_hides_outside_range() {
        let a = loc("a", "Town", 100.0, 100.0, 0.7);
        let all = vec![a];
        assert!(declutter_town_labels(&all, -4.0).is_empty());
        assert!(declutter_town_labels(&all, 3.0).is_empty());
        assert_eq!(declutter_town_labels(&all, -2.0).len(), 1);
    }
}
