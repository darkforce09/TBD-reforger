//! Role: importance tests.
//! Position: `symbology/labels/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::symbology::labels::importance::*;

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
    let a = loc("a", "Town", 100.0, 100.0, 0.7);
    let all = vec![a];
    assert!(declutter_town_labels(&all, -4.6).is_empty());
    assert!(declutter_town_labels(&all, 3.1).is_empty());
    assert_eq!(declutter_town_labels(&all, -2.0).len(), 1);

    assert_eq!(declutter_town_labels(&all, TOWN_LABEL_FADE_END).len(), 1);
}

#[test]
fn kind_filter_excludes_non_settlements() {
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
    let l = loc_kind("s", "Sawmill", 0.4, "locality");
    let all = vec![l.clone()];
    assert!(should_draw_town_label(&l, &all, 0.0));
    assert!(should_draw_town_label(&l, &all, 1.5));
    assert!(!should_draw_town_label(&l, &all, -0.5));
    assert!(!should_draw_town_label(&l, &all, -2.0));
}

#[test]
fn wide_band_importance_gate() {
    let z = -3.68;
    let capital = loc_kind("c", "Montignac", 0.85, "town");

    let minor = LocationLabel {
        x: 6000.0,
        y: 6000.0,
        ..loc_kind("m", "Provins", 0.55, "town")
    };
    let all = vec![capital.clone(), minor.clone()];
    assert!(should_draw_town_label(&capital, &all, z));
    assert!(!should_draw_town_label(&minor, &all, z));

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
