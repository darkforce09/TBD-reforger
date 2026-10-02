//! Role: the two-click capture, the sub-mode toggle, and the viewshed placement state.
//! Position: `editing/tools/line_of_sight/tests` in the map engine.
//! Signals & state: explicit profiles, shots and rasters built in the test body.
//! Invariants: a third click starts a fresh capture; escape steps down; a placement replaces rather than appends.

use super::*;
use crate::spatial::los::terrain::viewshed::Viewshed;
use crate::spatial::los::terrain::viewshed::Visibility;

#[test]
fn click_captures_observer_then_target() {
    let mut st = LosState::new();
    assert!(st.is_empty());
    // First click → observer pending, no shot yet, not "completed".
    assert!(!st.click(10.0, 20.0, Some(5.0)));
    assert_eq!(st.pending_obs, Some((10.0, 20.0, Some(5.0))));
    assert!(st.shot.is_none());
    assert!(!st.is_empty());
    // Second click → shot completed, pending cleared, returns true.
    assert!(st.click(110.0, 20.0, Some(8.0)));
    assert!(st.pending_obs.is_none());
    let shot = st.shot.expect("shot placed");
    assert_eq!(
        (shot.obs_x, shot.obs_y, shot.obs_z),
        (10.0, 20.0, Some(5.0))
    );
    assert_eq!(
        (shot.tgt_x, shot.tgt_y, shot.tgt_z),
        (110.0, 20.0, Some(8.0))
    );
    assert!((shot.distance_m() - 100.0).abs() < 1e-9);
}

#[test]
fn third_click_starts_a_fresh_capture() {
    let mut st = LosState::new();
    st.click(0.0, 0.0, None);
    st.click(100.0, 0.0, None);
    assert!(st.shot.is_some());
    // A third click retires the old shot and starts a new observer.
    assert!(!st.click(500.0, 500.0, None));
    assert_eq!(st.pending_obs, Some((500.0, 500.0, None)));
    assert!(
        st.shot.is_none(),
        "the previous shot is retired on a new capture"
    );
}

/// Esc mirrors the ruler's two-step (Decision 3): first drops the in-progress capture, a second
/// clears the placed result; an Esc with nothing acts false (falls through — never swallowed).
#[test]
fn escape_two_step_dismissal() {
    let mut st = LosState::new();
    assert!(!st.escape(), "nothing to dismiss → no act");
    // In-progress (observer placed, no target): first Esc drops it.
    st.click(0.0, 0.0, None);
    assert!(st.pending_obs.is_some() && st.shot.is_none());
    assert!(st.escape(), "first Esc acts");
    assert!(st.is_empty(), "first Esc drops the in-progress observer");
    // Placed shot: Esc clears it.
    st.click(0.0, 0.0, None);
    st.click(100.0, 0.0, None);
    assert!(st.shot.is_some());
    assert!(st.escape(), "Esc on a placed shot acts");
    assert!(st.is_empty(), "Esc clears the placed result");
    assert!(!st.escape(), "now empty → no act");
}

#[test]
fn clear_is_idempotent_and_total() {
    let mut st = LosState::new();
    st.click(0.0, 0.0, None);
    st.click(1.0, 0.0, None);
    st.clear();
    assert!(st.is_empty());
    st.clear(); // idempotent
    assert!(st.is_empty());
}

// ── the sub-mode toggle and the viewshed placement state ──────────────────────────────────────

/// Re-clicking the tool advances Ray -> Viewshed -> Ray. The default is the ray.
#[test]
fn los_mode_toggles_ray_and_viewshed() {
    assert_eq!(
        LosMode::default(),
        LosMode::Ray,
        "default sub-mode is the ray"
    );
    assert_eq!(LosMode::Ray.toggled(), LosMode::Viewshed);
    assert_eq!(LosMode::Viewshed.toggled(), LosMode::Ray);
    assert!(LosMode::Viewshed.is_viewshed());
    assert!(!LosMode::Ray.is_viewshed());
    // Two toggles return to the start (a genuine 2-cycle).
    assert_eq!(LosMode::Ray.toggled().toggled(), LosMode::Ray);
}

// ── T-644 — the viewshed state machine (one-click placement, dismissal) ──────────────────────

#[test]
fn viewshed_state_place_replaces_and_clears() {
    let mut st = ViewshedState::new();
    assert!(st.is_empty());
    // One click places the observer (raster left None for the host to fill).
    st.place(100.0, 200.0, Some(50.0));
    assert_eq!(st.observer, Some((100.0, 200.0, Some(50.0))));
    assert!(st.raster.is_none());
    assert!(!st.is_empty());
    // Host fills the raster.
    st.set_raster(Viewshed {
        cols: 1,
        rows: 1,
        cells: vec![Visibility::Visible],
        min_x: 0.0,
        min_y: 0.0,
        max_x: 0.0,
        max_y: 0.0,
        obs_x: 100.0,
        obs_y: 200.0,
    });
    assert!(st.raster.is_some());
    // A NEW placement replaces both observer and raster (a viewshed is one disc, not a chain).
    st.place(500.0, 500.0, None);
    assert_eq!(st.observer, Some((500.0, 500.0, None)));
    assert!(
        st.raster.is_none(),
        "re-placing retires the previous raster"
    );
}

#[test]
fn viewshed_escape_is_one_step() {
    let mut st = ViewshedState::new();
    assert!(!st.escape(), "nothing placed → no act");
    st.place(1.0, 2.0, None);
    assert!(st.escape(), "Esc on a placed observer acts");
    assert!(st.is_empty(), "Esc clears observer + raster in one step");
    assert!(!st.escape(), "now empty → no act");
}

#[test]
fn viewshed_clear_is_idempotent() {
    let mut st = ViewshedState::new();
    st.place(1.0, 2.0, Some(3.0));
    st.clear();
    assert!(st.is_empty());
    st.clear();
    assert!(st.is_empty());
}

/// Decision-4 pin extended: the viewshed state, like the ray, is session-local overlay state and
/// must never write the document. Covered by `the_line_of_sight_tool_never_writes_the_document`
/// (the whole-directory source scrub), but asserted here too so a future reader sees the viewshed
/// is in scope for that guarantee.
#[test]
fn viewshed_is_session_local_not_doc() {
    // The state struct holds only overlay data (observer point + raster) — no doc handle, no id.
    // A compile-time proof by construction; this test documents the intent and fails loudly if
    // someone adds a doc-mutating token to the module (the source scrub in
    // `the_line_of_sight_tool_never_writes_the_document`).
    let st = ViewshedState::default();
    assert!(
        st.is_empty(),
        "a fresh viewshed touches nothing (no doc, no map)"
    );
}
