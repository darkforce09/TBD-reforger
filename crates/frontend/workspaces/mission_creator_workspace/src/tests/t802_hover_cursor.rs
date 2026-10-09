use super::{
    HOVER_CURSOR_PICKABLE, HOVER_CURSOR_PLAIN, HOVER_RELEASE_PX, HOVER_THROTTLE_MS, HoverState,
    hover_cursor_css, hover_due, hover_next, hover_suppressed,
};

/* ── the state machine ─────────────────────────────────────────────────────────────────── */

/// The throttle is a FLOOR, and every degenerate clock resolves to "test it". A throttle that
/// silently stopped answering (a clock that went backwards across a page resume, a NaN out of a
/// singular timer) would present as the feature being off, which is the failure mode with no
/// symptom — so it fails open, not closed.
#[test]
fn the_throttle_is_a_floor_and_fails_open() {
    let fresh = HoverState::default();
    assert!(hover_due(fresh, 0.0), "never-tested must be due");
    assert!(hover_due(fresh, 1_000_000.0));

    let tested = HoverState {
        last_ms: 1000.0,
        ..HoverState::default()
    };
    assert!(!hover_due(tested, 1000.0), "same instant is not due");
    assert!(
        !hover_due(tested, 1000.0 + HOVER_THROTTLE_MS - 0.001),
        "one tick short of the window is not due"
    );
    assert!(
        hover_due(tested, 1000.0 + HOVER_THROTTLE_MS),
        "the window boundary is due"
    );
    assert!(
        hover_due(tested, 999.0),
        "a clock that went BACKWARDS must be due, not wedged shut"
    );
    assert!(hover_due(tested, f64::NAN), "a NaN clock must be due");
    assert!(hover_due(tested, f64::INFINITY));
}

/// **The churn acceptance, as a property.** A hand resting on the rim of a glyph produces an
/// ALTERNATING hit/miss stream inside a couple of pixels. Fold that whole stream and the cursor
/// must change exactly ONCE — on acquisition — and never again.
#[test]
fn jitter_on_a_glyph_rim_changes_the_cursor_exactly_once() {
    let (cx, cy) = (400.0, 300.0);
    let mut st = HoverState::default();
    let mut cur = hover_cursor_css(false);
    let mut changes = 0;
    // 60 ticks: hit / miss / miss / hit … all within one pixel of the anchor, which is well
    // inside the release band. This is the sequence a 4 px pick radius produces under tremor.
    for i in 0..60 {
        let hit = i % 3 == 0;
        let (px, py) = (cx + f64::from(i % 3) * 0.5, cy - f64::from(i % 2) * 0.4);
        st = hover_next(st, hit, px, py, f64::from(i) * HOVER_THROTTLE_MS);
        let next = hover_cursor_css(st.pickable);
        if next != cur {
            changes += 1;
            cur = next;
        }
    }
    assert_eq!(
        changes, 1,
        "T-802: hovering ONE entity must change the cursor once (default → pointer); the \
         acceptance calls 3+ changes churn"
    );
    assert_eq!(cur, HOVER_CURSOR_PICKABLE);
}

/// The dead-band is a band, not a latch: travelling off the glyph drops the claim on the first
/// miss past [`HOVER_RELEASE_PX`], and a held miss does NOT re-anchor (otherwise a slow drift
/// would carry "pickable" across the whole map, one sub-band step at a time).
#[test]
fn leaving_the_band_drops_the_claim_and_a_held_miss_never_re_anchors() {
    let (cx, cy) = (100.0, 100.0);
    let acquired = hover_next(HoverState::default(), true, cx, cy, 0.0);
    assert!(acquired.pickable);
    assert_eq!(acquired.anchor, Some((cx, cy)));

    // Inside the band: held, and the anchor is UNMOVED.
    let inside = hover_next(acquired, false, cx + HOVER_RELEASE_PX - 0.01, cy, 40.0);
    assert!(inside.pickable, "a miss inside the band holds the claim");
    assert_eq!(
        inside.anchor,
        Some((cx, cy)),
        "a held miss must not move the anchor"
    );

    // Walk outwards in sub-band steps. With a re-anchoring hold this would never end.
    let mut st = acquired;
    let mut x = cx;
    for _ in 0..12 {
        x += HOVER_RELEASE_PX - 0.5;
        st = hover_next(st, false, x, cy, 40.0);
    }
    assert!(
        !st.pickable,
        "T-802: a steady walk away from the glyph must escape the dead-band"
    );
    assert_eq!(st.anchor, None, "a dropped claim clears its anchor");

    // And a single decisive move past the band drops it immediately.
    let gone = hover_next(acquired, false, cx + HOVER_RELEASE_PX + 0.01, cy, 40.0);
    assert!(!gone.pickable);
    assert!(!hover_next(HoverState::default(), false, cx, cy, 40.0).pickable);
}

/// The two cursor values, and the reason `default` is written rather than left as `auto`.
#[test]
fn the_cursor_is_pointer_over_pickable_and_default_over_empty() {
    assert_eq!(hover_cursor_css(true), "pointer");
    assert_eq!(hover_cursor_css(false), "default");
    assert_eq!(HOVER_CURSOR_PICKABLE, "pointer");
    assert_eq!(
        HOVER_CURSOR_PLAIN, "default",
        "T-802: the resting value must be an ASSERTED `default`, not the UA `auto` — `auto` is \
         indistinguishable from never having asked, which is the O-8 defect itself"
    );
}

/// Suppression is the whole truth table: any one reason suppresses, and nothing else does.
#[test]
fn every_gesture_suppresses_the_hover_and_nothing_else_does() {
    assert!(
        !hover_suppressed(false, false, false),
        "idle pointer hovers"
    );
    for (g, p, m) in [
        (true, false, false),
        (false, true, false),
        (false, false, true),
        (true, true, true),
    ] {
        assert!(
            hover_suppressed(g, p, m),
            "T-802: gesture={g} place={p} measuring={m} must suppress the hover read"
        );
    }
}
