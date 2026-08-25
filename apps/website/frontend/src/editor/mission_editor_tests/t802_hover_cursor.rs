use super::{
    hover_cursor_css, hover_due, hover_next, hover_suppressed, HoverState, COMMENT_PICK_PX,
    HOVER_CURSOR_PICKABLE, HOVER_CURSOR_PLAIN, HOVER_RELEASE_PX, HOVER_THROTTLE_MS,
};
use crate::editor::arsenal::class_r_scrub::{live_code, only_body};

/// The editor page, scrubbed. Sliced from the RAW source at the component anchor first (the
/// `t784_comment_glyph::page` idiom): `live_code` truncates at the first `#[cfg(test)]`, and
/// this file has one before the component.
fn page() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(raw.matches(anchor.as_str()).count(), 1);
    live_code(&raw[raw.find(anchor.as_str()).expect("counted")..])
}

/// The T-802 primitives block, scrubbed — sliced the same way, from the first constant. T-934.10
/// moved the pure state machine to `canvas/render_sync.rs`, so the declarations are read there.
fn hover_block() -> String {
    let anchor = format!("pub(crate) const HOVER_CURSOR_{}", "PICKABLE");
    let raw = include_str!("../canvas/render_sync.rs");
    assert_eq!(raw.matches(anchor.as_str()).count(), 1);
    live_code(&raw[raw.find(anchor.as_str()).expect("counted")..])
}

/// The live body of `hover_hit`, scrubbed. `hover_hit` STAYED in `mission_editor.rs` at the
/// T-934.10 split (it reads OPS_CTX through `editor_ops::vehicle_points`, so it is not pure),
/// so this slices from its cache struct's anchor there.
fn hover_hit_body() -> String {
    let anchor = format!("pub(crate) struct Hover{}", "Points");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(raw.matches(anchor.as_str()).count(), 1);
    let block = live_code(&raw[raw.find(anchor.as_str()).expect("counted")..]);
    only_body(&block, &["pub(crate) fn hover_", "hit("].concat()).to_string()
}

/// The live `pointermove` closure, scrubbed. T-934.13 moved the gesture closures verbatim to
/// `canvas/gestures.rs`, so the anchor resolves there now (the page keeps `onpointerleave` and
/// the mount seed, which the mount/leave pin below still reads via `page()`).
fn pointermove() -> String {
    let src = live_code(include_str!("../canvas/gestures.rs"));
    let anchor = ["let onpointermove = ", "Closure::"].concat();
    assert_eq!(src.matches(anchor.as_str()).count(), 1);
    only_body(&src, &anchor).to_string()
}

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

/// The ticket's band, restated as an assertion so a later "tune" that makes this a per-frame
/// pick has to argue with a test rather than with a comment.
#[test]
fn the_throttle_is_inside_the_tickets_30_to_60_ms_band() {
    assert!(
        (30.0..=60.0).contains(&HOVER_THROTTLE_MS),
        "T-802: the hover throttle is {HOVER_THROTTLE_MS} ms — outside the 30–60 ms band the \
         ticket sets. Below 30 ms this is a per-frame pick again (the T-057 regression); above \
         60 ms the cursor visibly lags the pointer."
    );
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

/// **The hit radius keys off the pick radius, not off the glyph art.** T-808 is changing what
/// entities look like in the same wave; a hand-picked pixel count here would drift away from
/// what a click actually hits the moment those glyphs land.
#[test]
fn the_release_band_is_derived_from_the_pick_radius() {
    assert!(
        (HOVER_RELEASE_PX - COMMENT_PICK_PX * 1.5).abs() < f64::EPSILON,
        "T-802: HOVER_RELEASE_PX is {HOVER_RELEASE_PX}, expected 1.5 × the pick radius \
         ({COMMENT_PICK_PX})"
    );
    assert!(
        HOVER_RELEASE_PX > COMMENT_PICK_PX,
        "T-802: a dead-band no wider than the pick radius is not a dead-band"
    );
    // …and it must be WRITTEN as a derivation, not as a literal that happens to agree today.
    let decl = [
        "pub(crate) const HOVER_RELEASE_PX: f64 = ",
        "COMMENT_PICK_PX",
    ]
    .concat();
    assert!(
        hover_block().contains(&decl),
        "T-802: HOVER_RELEASE_PX must be declared in terms of COMMENT_PICK_PX (this file's \
         pinned restatement of MissionDocCore::PICK_RADIUS_PX), never as a fresh pixel guess"
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

/* ── the call site (the T-057 half) ────────────────────────────────────────────────────── */

/// **The throttle gates the pick, not the other way round.** The whole cost argument is that
/// most pointer moves do NO work; a pick hoisted above `hover_due` would answer every move and
/// re-create the regression that deleted hover picking at T-057.
#[test]
fn the_throttle_runs_before_the_pick_and_before_the_gesture_machine() {
    let body = pointermove();
    let (sup, due, hit, take) = (
        body.find("hover_suppressed("),
        body.find("hover_due("),
        body.find("hover_hit("),
        body.find("left.borrow_mut().take()"),
    );
    let (sup, due, hit, take) = (
        sup.expect("T-802: pointermove must ask hover_suppressed"),
        due.expect("T-802: pointermove must ask hover_due"),
        hit.expect("T-802: pointermove must run hover_hit"),
        take.expect("the gesture machine still takes `left`"),
    );
    assert!(
        sup < due,
        "T-802: suppression must be decided before the throttle is consulted"
    );
    assert!(
        due < hit,
        "T-802: the hit-test must sit BEHIND the throttle — a pick on every pointermove is the \
         T-057 cost that removed hover picking in the first place"
    );
    assert!(
        hit < take,
        "T-802: the hover read must run before the gesture machine takes `left`, so 'no \
         gesture is in flight' is still knowable"
    );
}

/// **Strictly read-only with respect to gesture state.** The hover block may ASK whether a
/// gesture is running; it may not take, replace or mutate one. (Its own `hover_points` cache is
/// a `borrow_mut`, which is why this looks for the `left` handle by name.)
#[test]
fn the_hover_read_never_touches_the_gesture_state() {
    let body = pointermove();
    let open = ["let now_ms = js_sys::", "Date::now()"].concat();
    assert_eq!(
        body.matches(open.as_str()).count(),
        1,
        "T-802: the hover block must open at exactly one throttle-clock read"
    );
    let start = body.find(open.as_str()).expect("counted");
    let end = body
        .find("left.borrow_mut().take()")
        .expect("the gesture machine");
    let block = &body[start..end];
    assert!(
        block.contains("left.borrow().is_some()"),
        "T-802: the gesture probe must be a SHARED borrow of `left`; body was:\n{block}"
    );
    for forbidden in ["left.borrow_mut", "left.take", "*left."] {
        assert!(
            !block.contains(forbidden),
            "T-802: the hover read must not `{forbidden}` — a hover test that mutates or \
             consumes gesture state is a defect (T-723 / T-795 / T-796 all live in this \
             handler); block was:\n{block}"
        );
    }
}

/// **One document read per generation.** The point sets are cached against `doc_tick` — the
/// counter `refresh_docks` bumps in the same commit tail that re-binds the glyph lanes — so the
/// hover can never be staler than the picture, and a 25 Hz `materialize()` never happens.
#[test]
fn the_point_sets_are_cached_against_the_lane_binding_tick() {
    let body = pointermove();
    assert!(
        body.contains("hover_hit(") && body.contains("doc_tick.get_untracked()"),
        "T-802: the hit-test must be keyed on doc_tick — the same channel the render lanes \
         bind on; body was:\n{body}"
    );
    let hit = hover_hit_body();
    assert!(
        hit.contains("c.tick != tick") && hit.contains("map_render_slot_soa"),
        "T-802: hover_hit must refresh its cache ONLY when the tick moved (via map_render_slot_soa); body was:\n{hit}"
    );
    assert!(
        body.contains("doc_tick.get_untracked()") && !body.contains("doc_tick.get()"),
        "T-802: the pointermove must read doc_tick UNTRACKED — a tracked read from inside a \
         DOM event closure would create a subscription this handler has no business owning"
    );
}

/// **No second point set and no second transform.** The hover asks the click path's own
/// question, through the click path's own functions, against the frozen camera the CUR read-out
/// already built — so "the cursor says pickable" and "the click picks" cannot disagree, and the
/// wave-201 third-transform-copy defect class is not re-opened.
#[test]
fn the_hover_reuses_the_click_paths_pick_and_camera() {
    let hit = hover_hit_body();
    let pick = ["pick_slot_or", "_vehicle("].concat();
    assert!(
        hit.contains(&pick),
        "T-802: the slot/vehicle half must be select_tool's own pick; body was:\n{hit}"
    );
    assert!(
        hit.contains(&["pick_", "comment("].concat()) && hit.contains("COMMENT_PICK_PX"),
        "T-802: the comment half must be the click path's pick_comment at its own tolerance"
    );
    assert!(
        hit.contains("unproject_xy(px, py)") && hit.contains("unproject_xy(px + COMMENT_PICK_PX"),
        "T-802: the comment tolerance must be derived by unprojecting two points \
         COMMENT_PICK_PX apart — the identical derivation the T-796 drag arm and the T-784 \
         click path use; body was:\n{hit}"
    );
    let body = pointermove();
    assert_eq!(
        body.matches("frozen_camera(").count(),
        1,
        "T-802: the pointermove must build the frozen camera ONCE and share it with the hover \
         hit-test — a second construction is a second copy of the transform"
    );
}

/// **Markers are deliberately absent.** They have no selection route at all, so a pointer
/// cursor over one would be the `cursor-pointer`-over-a-dead-click lie T-754 was filed for.
#[test]
fn the_hover_never_claims_a_marker_is_pickable() {
    let hit = hover_hit_body();
    assert!(
        !hit.to_lowercase().contains("marker"),
        "T-802: a marker has no selection route — claiming one is pickable is the T-754 lie \
         with the sign flipped; body was:\n{hit}"
    );
}

/// **The style write happens on a TRANSITION.** 25 style mutations a second for an unchanged
/// value is exactly the "cursor on the render path" shape T-057 removed.
#[test]
fn the_cursor_is_written_only_when_the_verdict_changes() {
    let body = pointermove();
    assert!(
        body.contains("if next.pickable != prev.pickable"),
        "T-802: the cursor write must be guarded by a change comparison; body was:\n{body}"
    );
    let set = ["set_map_", "cursor("].concat();
    assert_eq!(
        body.matches(set.as_str()).count(),
        2,
        "T-802: the pointermove writes the cursor in exactly two places — the suppression \
         reset and the transition"
    );
}

/// The resting cursor is asserted at MOUNT, and dropped when the pointer leaves the map. Both
/// go through the one writer, so there is a single source for the value.
#[test]
fn the_mount_seeds_the_resting_cursor_and_pointerleave_drops_the_claim() {
    let page = page();
    let mount = only_body(&page, "canvas_ref.on_load(");
    let set = ["set_map_", "cursor("].concat();
    assert!(
        mount.contains(&format!("{set}&canvas, false)")),
        "T-802: the mount must assert the resting cursor, or 'over empty ground' reads `auto` \
         until the first miss"
    );
    let leave = only_body(&page, &["let onpointerleave = ", "Closure::"].concat());
    assert!(
        leave.contains("HoverState::default()") && leave.contains(&set),
        "T-802: pointerleave must reset the hover state AND the cursor — `cursor` is an \
         inherited property, so a stranded `pointer` follows the pointer onto the chrome; \
         body was:\n{leave}"
    );
}

/// Hollow canary: the source pins above are load-bearing, not decorative. Stripping the needle
/// from an in-memory copy of the real source breaks the assertion that found it.
#[test]
fn the_call_site_pins_are_load_bearing() {
    let body = pointermove();
    for needle in [
        "hover_due(",
        "hover_hit(",
        "left.borrow().is_some()",
        "if next.pickable != prev.pickable",
        "doc_tick.get_untracked()",
    ] {
        assert!(
            body.contains(needle),
            "T-802: `{needle}` must be present in the live pointermove for its pin to mean \
             anything"
        );
        assert!(
            !body.replace(needle, "").contains(needle),
            "T-802: `{needle}` must be findable exactly by the pin's own needle"
        );
    }
}
