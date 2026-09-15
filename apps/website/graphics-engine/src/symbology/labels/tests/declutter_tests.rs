//! Role: declutter tests.
//! Position: `symbology/labels/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::symbology::labels::declutter::*;

fn lab(id: u32, x: i32, y: i32, imp: u16, text: &str) -> LabelSpec {
    LabelSpec {
        id,
        x,
        y,
        importance: imp,
        text: text.to_string(),
    }
}

#[test]
fn empty_input_yields_empty() {
    assert!(declutter(&[], 0.0).is_empty());
}

#[test]
fn blank_text_dropped() {
    let out = declutter(&[lab(1, 0, 0, 10, "  ")], 0.0);
    assert!(out.is_empty());
}

#[test]
fn far_apart_both_kept() {
    let z = 0.0;
    let d = min_label_distance_m(z);
    let a = lab(1, 0, 0, 1, "A");
    let b = lab(2, (d as i32) + 10, 0, 1, "B");
    let out = declutter(&[a, b], z);
    assert_eq!(out.len(), 2);
    assert!(declutter_invariant_holds(&out, z));
}

#[test]
fn close_pair_keeps_higher_importance() {
    let z = 0.0;
    let low = lab(1, 0, 0, 1, "low");
    let high = lab(2, 1, 0, 99, "high");
    let out = declutter(&[low, high], z);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].text, "high");
    assert!(declutter_invariant_holds(&out, z));
}

#[test]
fn g4_invariant_on_randomish_fixture() {
    let z = -2.0;
    let labels = vec![
        lab(1, 100, 100, 50, "A"),
        lab(2, 105, 100, 40, "B"),
        lab(3, 5000, 5000, 10, "C"),
        lab(4, 5010, 5000, 90, "D"),
        lab(5, 8000, 100, 5, "E"),
    ];
    let out = declutter(&labels, z);
    assert!(declutter_invariant_holds(&out, z));
    assert!(out.iter().any(|l| l.text == "A" || l.text == "D"));

    assert!(!out.iter().any(|l| l.text == "B"));

    assert!(!out.iter().any(|l| l.text == "C"));
}

#[test]
fn min_distance_scales_with_zoom() {
    assert!((min_label_distance_m(0.0) - 48.0).abs() < 1e-9);
    assert!((min_label_distance_m(-1.0) - 96.0).abs() < 1e-9);
    assert!((min_label_distance_m(1.0) - 24.0).abs() < 1e-9);
}
