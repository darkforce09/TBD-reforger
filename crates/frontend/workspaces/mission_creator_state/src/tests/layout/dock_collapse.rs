//! The dock collapse state machine, the inset accessors, and the centre-hold math: the policy the
//! wasm keydown and the dock chevrons drive, factored into pure functions a native test executes.

use super::{
    DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX, STUB_PX, TOOLBELT_BAND_PX, centre_hold_target,
    dock_left_collapsed, dock_left_px, dock_right_collapsed, dock_right_px, pane_center_px,
    set_chrome_hidden, set_dock_left_collapsed, set_dock_right_collapsed, strip_top_px,
    toolbelt_band_px,
};
use camera_math::ortho::state::OrthoCamera;

/// Reset the three thread-local latches so tests don't leak state into one another (they run on
/// the same thread). Every test that touches the accessors starts here.
fn reset() {
    set_dock_left_collapsed(false);
    set_dock_right_collapsed(false);
    set_chrome_hidden(false);
}

/// The accessors fold collapse + chrome_hidden into the live inset exactly as documented:
/// expanded → the const; collapsed → the 24×24 stub; hidden → 0 (full-bleed), regardless of the
/// per-dock latch.
#[test]
fn accessors_fold_collapse_and_hidden() {
    reset();
    // Expanded, shown.
    assert_eq!(dock_left_px(), DOCK_LEFT_PX);
    assert_eq!(dock_right_px(), DOCK_RIGHT_PX);
    assert_eq!(strip_top_px(), STRIP_TOP_PX);
    assert_eq!(toolbelt_band_px(), TOOLBELT_BAND_PX);

    // Collapse each dock in turn — only its own inset shrinks to the stub; strip/band unchanged.
    set_dock_left_collapsed(true);
    assert_eq!(dock_left_px(), STUB_PX);
    assert_eq!(
        dock_right_px(),
        DOCK_RIGHT_PX,
        "left collapse must not touch right"
    );
    assert_eq!(
        strip_top_px(),
        STRIP_TOP_PX,
        "docks collapse; the strip does not"
    );
    assert_eq!(toolbelt_band_px(), TOOLBELT_BAND_PX);
    set_dock_right_collapsed(true);
    assert_eq!(dock_right_px(), STUB_PX);

    // chrome_hidden WINS: every inset reports 0 while active, even though both docks are still
    // latched collapsed underneath.
    set_chrome_hidden(true);
    assert_eq!(dock_left_px(), 0.0);
    assert_eq!(dock_right_px(), 0.0);
    assert_eq!(strip_top_px(), 0.0);
    assert_eq!(toolbelt_band_px(), 0.0);
    // …and the collapse latches PERSIST underneath (orthogonal states).
    assert!(dock_left_collapsed() && dock_right_collapsed());

    // Un-hide: the persisted collapse state re-applies (stub, not full) — the hide/show cycle
    // never clobbered it.
    set_chrome_hidden(false);
    assert_eq!(dock_left_px(), STUB_PX);
    assert_eq!(dock_right_px(), STUB_PX);
    reset();
}

/// The `E`/`R` state machine, modelled as the exact toggle the keydown arms run: E flips only the
/// left latch, R only the right; each is its own independent toggle. Perturbation: were E to write
/// the right latch (a copy-paste swap), the "R untouched by E" assertion fails.
#[test]
fn e_toggles_left_r_toggles_right_independently() {
    reset();
    // E: left off→on, right untouched.
    set_dock_left_collapsed(!dock_left_collapsed());
    assert!(dock_left_collapsed(), "E collapses the left dock");
    assert!(!dock_right_collapsed(), "E must not touch the right dock");
    // R: right off→on, left still on.
    set_dock_right_collapsed(!dock_right_collapsed());
    assert!(dock_right_collapsed(), "R collapses the right dock");
    assert!(dock_left_collapsed(), "R must not touch the left dock");
    // E again: left on→off (a toggle, not a one-way set), right still on.
    set_dock_left_collapsed(!dock_left_collapsed());
    assert!(!dock_left_collapsed(), "E again expands the left dock");
    assert!(dock_right_collapsed());
    reset();
}

/// The map-pane centre is the midpoint of the chrome-free rect using the LIVE insets. Collapsing a
/// dock moves the pane centre toward that side by half the freed width — the delta the centre-hold
/// consumes.
#[test]
fn pane_centre_uses_live_insets() {
    reset();
    let (w, h) = (1920.0, 1080.0);
    let full = pane_center_px(w, h);
    assert!((full.0 - (DOCK_LEFT_PX + (w - DOCK_RIGHT_PX)) / 2.0).abs() < 1e-9);
    assert!((full.1 - (STRIP_TOP_PX + (h - TOOLBELT_BAND_PX)) / 2.0).abs() < 1e-9);

    // Collapse the left dock: its inset drops DOCK_LEFT_PX→STUB_PX, so the pane centre shifts LEFT
    // by half that change.
    set_dock_left_collapsed(true);
    let left_col = pane_center_px(w, h);
    let expected_dx = (STUB_PX - DOCK_LEFT_PX) / 2.0; // negative → leftward
    assert!(
        (left_col.0 - (full.0 + expected_dx)).abs() < 1e-9,
        "pane centre must shift by half the freed left width"
    );
    assert!(
        (left_col.1 - full.1).abs() < 1e-9,
        "y is unaffected by a dock collapse"
    );
    reset();
}

/// CENTRE-HOLD, fired against the engine's OWN camera (camera_math `OrthoCamera` — the exact
/// type `select_tool::frozen_camera` builds). With the nudge applied, the world point under the
/// pane centre is INVARIANT across the collapse reflow (RESTORE); without it, that point MOVES by
/// the pane-centre delta in world units (FAIL) — so the assertion is not vacuously true.
#[test]
fn centre_hold_keeps_the_pane_centre_world_point() {
    reset();
    let (w, h) = (1920.0, 1080.0);
    let (tx, ty, zoom) = (6400.0, 6400.0, -2.0);
    // Full-bleed camera: viewport IS the whole window (matches the editor + `frozen_camera`).
    let cam0 = OrthoCamera::new(w, h, tx, ty, zoom);
    let scale = zoom.exp2();

    // World point under the pane centre BEFORE the collapse.
    let before = pane_center_px(w, h);
    let held = cam0.unproject_xy(before.0, before.1);

    // Collapse the left dock → the pane centre moves.
    set_dock_left_collapsed(true);
    let after = pane_center_px(w, h);
    assert!(
        (after.0 - before.0).abs() > 1.0,
        "the reflow must actually move the pane centre or the test proves nothing"
    );

    // RESTORE: nudge the target, rebuild the (same-size) camera, and the held world point is back
    // under the NEW pane centre to sub-pixel world precision.
    let (nx, ny) = centre_hold_target(tx, ty, scale, before, after);
    let cam1 = OrthoCamera::new(w, h, nx, ny, zoom);
    let after_hold = cam1.unproject_xy(after.0, after.1);
    assert!(
        (after_hold[0] - held[0]).abs() < 1e-6 && (after_hold[1] - held[1]).abs() < 1e-6,
        "centre-hold: the pane-centre world point must be invariant across the reflow (got {after_hold:?}, want {held:?})"
    );

    // FAIL (perturbation): WITHOUT the nudge, the same-camera reflow shifts the pane-centre world
    // point by exactly the freed half-width in world units — proving the hold is load-bearing.
    let no_hold = cam0.unproject_xy(after.0, after.1);
    let drift = (no_hold[0] - held[0]).abs();
    let expected_drift = ((after.0 - before.0) / scale).abs();
    assert!(
        drift > 1e-6,
        "without centre-hold the world point MUST move — else the hold is untested"
    );
    assert!(
        (drift - expected_drift).abs() < 1e-6,
        "the un-held drift must equal the pane-centre delta / scale ({drift} vs {expected_drift})"
    );
    reset();
}

/// Degenerate guards: a non-positive scale (impossible for a live camera) is a no-op nudge, and an
/// unchanged pane centre yields no move.
#[test]
fn centre_hold_degenerate_is_a_noop() {
    assert_eq!(
        centre_hold_target(10.0, 20.0, 0.0, (1.0, 2.0), (3.0, 4.0)),
        (10.0, 20.0)
    );
    assert_eq!(
        centre_hold_target(10.0, 20.0, 4.0, (5.0, 6.0), (5.0, 6.0)),
        (10.0, 20.0),
        "no pane-centre change → no target move"
    );
}
