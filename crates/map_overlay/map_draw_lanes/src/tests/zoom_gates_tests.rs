//! Cases of the zoom gates: class visibility thresholds and the contour interval ladder rising
//! with metres per pixel.

use crate::zoom_gates::*;

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

    assert!(class_visible("forestFill", -0.1));
    assert!(!class_visible("forestFill", 0.0));
    assert!(!class_visible("forestFill", 1.0));
    assert!(class_visible("forestOutline", -1.5));
    assert!(!class_visible("forestOutline", -1.6));
    assert!(!class_visible("forestOutline", 0.0));
}

#[test]
fn fence_pier_gate_boundaries() {
    assert!(class_visible("fence", 1.5));
    assert!(!class_visible("fence", 1.49));
    assert!(class_visible("pier", -1.0));
    assert!(!class_visible("pier", -1.01));

    assert!(class_visible("prop", 3.0));
    assert!(!class_visible("prop", 2.99));

    assert!(!WORLD_RENDER_CLASSES.contains(&"fence"));
    assert!(!WORLD_RENDER_CLASSES.contains(&"pier"));
}

#[test]
fn interval_monotone_in_m_per_px() {
    let mut prev = 0.0;
    let mut mpp = 0.5;
    while mpp <= 80.0 {
        let interval = contour_interval_for_zoom(mpp);
        assert!(
            interval >= prev,
            "interval dropped from {prev} to {interval} at m/pix {mpp:.3}"
        );
        prev = interval;
        mpp *= 1.05;
    }
}
