//! Role: the polyline capture: append, end, dedup, and the escalating dismissal.
//! Position: `editing/tools/ruler/tests` in the map engine.
//! Signals & state: explicit vertices and chains built in the test body.
//! Invariants: escape drops the in-progress tail first and the placed points second; clear is idempotent and total.

use super::*;

// ── tool-mode arbitration (button filter + Select passthrough) ──────────────────────────────

#[test]
fn press_appends_and_arms_drawing() {
    let mut c = RulerChain::new();
    assert!(c.is_empty() && !c.drawing);
    c.press(0.0, 0.0, None);
    assert_eq!(c.points.len(), 1);
    assert!(c.drawing, "first press arms drawing");
    c.press(100.0, 0.0, Some(5.0));
    assert_eq!(c.points.len(), 2);
    assert_eq!(c.legs().len(), 1);
    assert!((c.total_m() - 100.0).abs() < 1e-9);
}

#[test]
fn double_click_ends_but_keeps_placed() {
    let mut c = RulerChain::new();
    c.press(0.0, 0.0, None);
    c.press(100.0, 0.0, None);
    c.press(100.0, 100.0, None);
    c.double_click();
    assert!(!c.drawing, "dbl-click ends drawing");
    assert_eq!(
        c.points.len(),
        3,
        "dbl-click KEEPS the placed points (Decision 3)"
    );
    assert_eq!(c.legs().len(), 2);
}

#[test]
fn dedup_tail_removes_coincident_final_vertex() {
    let mut c = RulerChain::new();
    c.press(0.0, 0.0, None);
    c.press(100.0, 0.0, None);
    c.press(100.0, 0.0, None); // dbl-click's coincident second press
    assert!(c.dedup_tail(0.5), "coincident tail removed");
    assert_eq!(c.points.len(), 2);
    // A genuine distinct tail is NOT removed.
    c.press(100.0, 50.0, None);
    assert!(!c.dedup_tail(0.5));
    assert_eq!(c.points.len(), 3);
}

/// Esc is the two-step escalating dismissal (Decision 3): first drops the in-progress tail
/// (keeps a legged measure), a second Esc clears the placed points; a lone un-legged vertex
/// clears on the first Esc.
#[test]
fn escape_two_step_dismissal() {
    let mut c = RulerChain::new();
    // Nothing to dismiss.
    assert!(!c.escape());
    // Multi-vertex, drawing → first Esc keeps points, stops drawing.
    c.press(0.0, 0.0, None);
    c.press(100.0, 0.0, None);
    assert!(c.drawing);
    assert!(c.escape(), "first Esc acts");
    assert!(!c.drawing, "first Esc stops drawing");
    assert_eq!(c.points.len(), 2, "first Esc KEEPS the legged measure");
    // Second Esc (now placed / not drawing) → clears.
    assert!(c.escape(), "second Esc acts");
    assert!(c.is_empty(), "second Esc clears the placed ruler");
    // A lone un-legged vertex clears on the FIRST Esc (nothing worth keeping).
    c.press(5.0, 5.0, None);
    assert_eq!(c.points.len(), 1);
    assert!(c.escape());
    assert!(c.is_empty(), "lone vertex clears on first Esc");
}

#[test]
fn clear_is_idempotent_and_total() {
    let mut c = RulerChain::new();
    c.press(0.0, 0.0, None);
    c.press(1.0, 0.0, None);
    c.clear();
    assert!(c.is_empty() && !c.drawing);
    c.clear(); // idempotent
    assert!(c.is_empty());
}

#[test]
fn status_readout_shows_total_and_last_leg() {
    let mut c = RulerChain::new();
    assert_eq!(c.status_readout(), None, "no legs → no readout");
    c.press(0.0, 0.0, Some(100.0));
    assert_eq!(c.status_readout(), None, "lone vertex → no readout");
    c.press(0.0, 412.0, Some(108.0));
    let s = c.status_readout().unwrap();
    assert!(
        s.starts_with("Σ 412 m · last "),
        "readout leads with the total: {s}"
    );
    assert!(
        s.contains("412 m · 000.0° · +8 m (2%)"),
        "readout carries the last-leg label: {s}"
    );
    // A second leg updates the total and swaps "last" to the newest leg. The 1000 m leg renders
    // in km per the leg formatter (≥1000 m → "1.00 km"), matching the Σ-total's km form.
    c.press(1000.0, 412.0, Some(108.0));
    let s2 = c.status_readout().unwrap();
    assert!(
        s2.starts_with("Σ 1.41 km · last "),
        "total accumulates: {s2}"
    );
    assert!(
        s2.contains("1.00 km · 090.0°"),
        "last-leg swaps to the new leg: {s2}"
    );
}

// ── label keying (T-727 world-coord key pin) ────────────────────────────────────────────────
