//! Role: label keying and the screen projection of a chain.
//! Position: `editing/tools/ruler/tests` in the map engine.
//! Signals & state: explicit vertices and chains built in the test body.
//! Invariants: a drawn node's key is WHERE the leg is, not what it reads, so identical labels on different legs never collide.

use super::*;

/// Two DIFFERENT legs that happen to share an identical LABEL STRING must get DIFFERENT keys: a
/// list keyed on text retains a stale node at the wrong position. The key is the leg's world
/// endpoints, so a node is tied to WHERE it is.
#[test]
fn label_keys_are_world_coords_not_text() {
    // Two legs with the same length + bearing → the SAME label string, at DIFFERENT places.
    let mut c = RulerChain::new();
    c.press(0.0, 0.0, None);
    c.press(0.0, 100.0, None); // leg 1: 100 m due north
    c.press(500.0, 100.0, None); // leg 2 (east), then…
    c.press(500.0, 200.0, None); // leg 3: 100 m due north — identical label to leg 1
    let legs = c.legs();
    let l1 = &legs[0];
    let l3 = &legs[2];
    assert_eq!(
        l1.label(),
        l3.label(),
        "the two north legs share a label string"
    );
    let identity = |x: f64, y: f64| (x, y); // identity projector for the pure test
    let proj = project_legs(&c, identity);
    // Same label, DIFFERENT keys — the whole point of keying by world coordinate.
    assert_eq!(proj[0].label, proj[2].label);
    assert_ne!(
        proj[0].key, proj[2].key,
        "T-727: legs with identical labels must have distinct world-coordinate keys"
    );
    // All keys unique across the chain.
    let mut keys: Vec<String> = proj.iter().map(|p| p.key.clone()).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), proj.len(), "every leg key is unique");
}

#[test]
fn world_key_quantises_and_distinguishes() {
    // Sub-0.1 m jitter maps to the SAME key (no per-frame churn)…
    assert_eq!(
        world_key(10.02, 20.0, 30.0, 40.0),
        world_key(10.03, 20.0, 30.0, 40.0)
    );
    // …but a genuine 1 m move is a different key.
    assert_ne!(
        world_key(10.0, 20.0, 30.0, 40.0),
        world_key(11.0, 20.0, 30.0, 40.0)
    );
}

#[test]
fn project_legs_maps_endpoints_and_midpoint() {
    let mut c = RulerChain::new();
    c.press(0.0, 0.0, None);
    c.press(100.0, 200.0, None);
    // A projector that scales by 2 and offsets — checks endpoints AND midpoint pass through it.
    let proj = project_legs(&c, |x, y| (x * 2.0 + 5.0, y * 2.0 + 7.0));
    assert_eq!(proj.len(), 1);
    let l = &proj[0];
    assert!((l.x1 - 5.0).abs() < 1e-9 && (l.y1 - 7.0).abs() < 1e-9);
    assert!((l.x2 - 205.0).abs() < 1e-9 && (l.y2 - 407.0).abs() < 1e-9);
    // Midpoint world (50,100) → (105, 207).
    assert!((l.mid_x - 105.0).abs() < 1e-9 && (l.mid_y - 207.0).abs() < 1e-9);
}

// ── Decision 4 pin: rulers are session-local overlay state, NEVER doc/store writes ───────────
