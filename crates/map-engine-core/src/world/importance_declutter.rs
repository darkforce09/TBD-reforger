//! A3-style importance-distance town label declutter (T-152.8).
//! Predicate: draw iff `nearest_more_important_m ≥ IMPORTANCE_SCALE · size_land_m · 2^(−z)`.

#![forbid(unsafe_code)]

/// A3 `uiMap.cpp` scale factor (T-152.8 L3).
pub const IMPORTANCE_SCALE: f64 = 0.08;
/// Base land footprint meters at importance = 1 (verify log pins).
pub const TOWN_BASE_SIZE_M: f64 = 400.0;
/// Absolute wide-zoom floor — labels hidden when zoomed out past this deck zoom
/// (T-152.17: widened −3.0 → −4.5 so island-fit zoom still shows the big names).
pub const TOWN_LABEL_MIN_ZOOM: f64 = -4.5;
/// Below this deck zoom the lane is importance-gated (only the biggest towns draw at the
/// widest band — T-152.17 wide-band gate).
pub const TOWN_LABEL_WIDE_ZOOM: f64 = -3.0;
/// Minimum `importance` to draw in the wide band `[MIN, WIDE)` (T-152.17: capitals only).
pub const TOWN_LABEL_WIDE_MIN_IMPORTANCE: f64 = 0.70;
/// Fade start — full alpha at/below this deck zoom, fading in past it (T-152.17 M2 = fade).
pub const TOWN_LABEL_MAX_ZOOM: f64 = 2.0;
/// Fade end / draw ceiling — alpha reaches 0 here; nothing drawn above (T-152.17).
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

/// Whether `kind` belongs on the settlement-only town lane at `deck_zoom` (T-152.17 lane policy).
///
/// `town`/`village`/`airport` always qualify; `locality` (sawmills, farms — map-real but not
/// towns) draws small at `z ≥ 0` only; every other kind (peak/hill/natural/…) is excluded — those
/// carry height labels on the T-152.16 heights lane. Absent `kind` defaults to `town`.
#[must_use]
pub fn town_lane_kind_ok(kind: Option<&str>, deck_zoom: f64) -> bool {
    match kind.unwrap_or("town") {
        "town" | "village" | "airport" => true,
        "locality" => deck_zoom >= 0.0,
        _ => false,
    }
}

/// Lane alpha multiplier at `deck_zoom`: `1.0` at/below `MAX_ZOOM`, linear to `0.0` at `FADE_END`,
/// `0.0` above (T-152.17 fade — baked into the pack tint alpha). With fade disabled it is a hard
/// step at `MAX_ZOOM`.
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

/// Whether `loc` passes the settlement-lane predicate at `deck_zoom`
/// (band + kind filter + wide-band importance gate + A3 declutter — T-152.17).
#[must_use]
pub fn should_draw_town_label(loc: &LocationLabel, all: &[LocationLabel], deck_zoom: f64) -> bool {
    if !(TOWN_LABEL_MIN_ZOOM..=town_label_zoom_ceiling()).contains(&deck_zoom) {
        return false;
    }
    // Lane policy: settlements only (hills/peaks/natural live on the T-152.16 heights lane).
    if !town_lane_kind_ok(loc.kind.as_deref(), deck_zoom) {
        return false;
    }
    // Wide-band gate: at the widest zoom only the biggest towns draw.
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

    fn loc_kind(id: &str, name: &str, imp: f64, kind: &str) -> LocationLabel {
        LocationLabel {
            kind: Some(kind.into()),
            ..loc(id, name, 100.0, 100.0, imp)
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
        // T-152.17 band [-4.5, 3.0]: out below the floor and above the fade ceiling.
        let a = loc("a", "Town", 100.0, 100.0, 0.7);
        let all = vec![a];
        assert!(declutter_town_labels(&all, -4.6).is_empty());
        assert!(declutter_town_labels(&all, 3.1).is_empty());
        assert_eq!(declutter_town_labels(&all, -2.0).len(), 1);
        // Fade end is inclusive — still in the draw set (alpha 0 handled at pack time).
        assert_eq!(declutter_town_labels(&all, TOWN_LABEL_FADE_END).len(), 1);
    }

    #[test]
    fn kind_filter_excludes_non_settlements() {
        // Peak / hill / natural never draw on the town lane; settlements do.
        let town = loc_kind("t", "Town", 0.6, "town");
        let village = loc_kind("v", "Village", 0.6, "village");
        let airport = loc_kind("a", "Airport", 0.6, "airport");
        let peak = loc_kind("p", "Peak", 0.6, "peak");
        let hill = loc_kind("h", "Hill", 0.6, "hill");
        let natural = loc_kind("n", "Rock", 0.6, "natural");
        for (l, ok) in [
            (&town, true),
            (&village, true),
            (&airport, true),
            (&peak, false),
            (&hill, false),
            (&natural, false),
        ] {
            let all = vec![l.clone()];
            assert_eq!(
                should_draw_town_label(l, &all, 0.0),
                ok,
                "kind {:?} lane membership",
                l.kind
            );
        }
    }

    #[test]
    fn locality_only_at_zoom_ge_0() {
        // Sub-features (localities) draw small at z >= 0 only, never at wide zoom.
        let l = loc_kind("s", "Sawmill", 0.4, "locality");
        let all = vec![l.clone()];
        assert!(should_draw_town_label(&l, &all, 0.0));
        assert!(should_draw_town_label(&l, &all, 1.5));
        assert!(!should_draw_town_label(&l, &all, -0.5));
        assert!(!should_draw_town_label(&l, &all, -2.0));
    }

    #[test]
    fn wide_band_importance_gate() {
        // Below WIDE_ZOOM (island fit z≈-3.68) only importance >= 0.70 towns draw.
        let z = -3.68;
        let capital = loc_kind("c", "Montignac", 0.85, "town");
        // Far from the capital so declutter never drops it (isolate the wide-band gate).
        let minor = LocationLabel {
            x: 6000.0,
            y: 6000.0,
            ..loc_kind("m", "Provins", 0.55, "town")
        };
        let all = vec![capital.clone(), minor.clone()];
        assert!(should_draw_town_label(&capital, &all, z));
        assert!(!should_draw_town_label(&minor, &all, z));
        // At z=-2 (>= WIDE_ZOOM) the gate is off; the minor town draws.
        assert!(should_draw_town_label(&minor, &all, -2.0));
    }

    #[test]
    fn fade_alpha_endpoints() {
        assert!((town_label_fade_alpha(1.0) - 1.0).abs() < 1e-9);
        assert!((town_label_fade_alpha(TOWN_LABEL_MAX_ZOOM) - 1.0).abs() < 1e-9);
        assert!((town_label_fade_alpha(2.5) - 0.5).abs() < 1e-9);
        assert!((town_label_fade_alpha(TOWN_LABEL_FADE_END) - 0.0).abs() < 1e-9);
        assert!((town_label_fade_alpha(3.5) - 0.0).abs() < 1e-9);
    }
}
