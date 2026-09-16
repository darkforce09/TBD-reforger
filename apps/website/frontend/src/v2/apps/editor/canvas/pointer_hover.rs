//! The map canvas's hover-cursor policy: the pure state machine behind "this pixel is pickable".
//!
//! Tab-local by construction. Nothing here survives a reload, and nothing here is a document
//! concept — it is one browser tab's cursor affordance, so it stays in the app while the picks it
//! consults live in the map engine.
//!
//! Hover picking is cheap only because four things keep it cheap: the hit-test is throttled to at
//! most one answer per [`HOVER_THROTTLE_MS`]; it introduces no geometry of its own, asking the
//! click path's own pick at the click path's own tolerance; the point sets are materialised once
//! per document generation and reused; and the cursor is written on a TRANSITION rather than per
//! tick. A hover pick on every pointer move is what took an earlier editor to single-digit frame
//! rates, which is why the affordance was absent until this machine existed.
//!
//! The machine is READ-ONLY with respect to the gesture state it runs beside: it asks whether a
//! gesture is in flight and is suppressed while one is ([`hover_suppressed`]); the only state it
//! owns is its own [`HoverState`].
// Every shipping caller is a `#[cfg(target_arch = "wasm32")]` closure and every other caller is a
// `#[cfg(test)]` pin, so the native non-test build reaches none of this.
#![allow(dead_code)]

use website_map_engine::editing::lanes::comments::COMMENT_PICK_PX;

/// The CSS `cursor` over a pickable entity. `pointer`, not `grab`, because it is the vocabulary
/// the rest of this editor already speaks: every clickable chrome row wears `cursor-pointer`, and
/// "wears pointer" means exactly "a click here resolves".
pub(crate) const HOVER_CURSOR_PICKABLE: &str = "pointer";

/// The CSS `cursor` everywhere else on the map. Written EXPLICITLY rather than left as the user
/// agent's `auto`, so the resting state is a value the surface asserts and a scripted read can
/// distinguish "decided: nothing here" from "never asked".
pub(crate) const HOVER_CURSOR_PLAIN: &str = "default";

/// The hover hit-test throttle floor, in milliseconds. 40 ms means at most 25 tests a second
/// against a pointer that fires two to five times that often: fast enough that the cursor changes
/// within one frame of arriving over a glyph, slow enough that the pick is off the pointer's hot
/// path.
pub(crate) const HOVER_THROTTLE_MS: f64 = 40.0;

/// The hysteresis dead-band, in SCREEN pixels: how far the pointer must travel from the pixel that
/// last HIT before a miss is believed.
///
/// Derived from the comment pick radius rather than from a hand-picked pixel count, so it tracks
/// what a click actually hits instead of what the glyph art happens to look like. One and a half
/// times the radius: wide enough that hand tremor at a glyph's rim cannot strobe the cursor,
/// narrow enough that leaving the glyph reads as instant.
pub(crate) const HOVER_RELEASE_PX: f64 = COMMENT_PICK_PX * 1.5;

/// Everything the hover cursor remembers between pointer moves. `Copy`, three scalars, and held in
/// a plain `Cell` beside the gesture state it must never touch.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct HoverState {
    /// Timestamp of the last hit-test, in the page clock's millisecond domain — the throttle
    /// clock. `0.0` (the `Default`) means "never tested", which [`hover_due`] treats as due.
    pub last_ms: f64,
    /// Is the cursor currently CLAIMING that the pixel under the pointer is pickable?
    pub pickable: bool,
    /// The pixel of the most recent HIT — the anchor the hysteresis dead-band is measured from.
    /// `None` whenever `pickable` is false; the two always move together.
    pub anchor: Option<(f64, f64)>,
}

/// May the hit-test run at `now_ms`? The throttle, as a pure question.
///
/// Due when at least [`HOVER_THROTTLE_MS`] has passed, when the clock has never been read
/// (`last_ms == 0.0`), and when the clock went BACKWARDS or non-finite — a stalled throttle that
/// silently stopped answering would look exactly like the feature being off, so every degenerate
/// clock resolves to "test it".
#[must_use]
pub(crate) fn hover_due(prev: HoverState, now_ms: f64) -> bool {
    if !now_ms.is_finite() || prev.last_ms <= 0.0 {
        return true;
    }
    !(0.0..HOVER_THROTTLE_MS).contains(&(now_ms - prev.last_ms))
}

/// Fold one hit-test result into the hover state. **The only place the pointer/plain decision is
/// made**, and pure so the hysteresis is provable off-target.
///
/// A HIT always claims pickable and re-anchors. A MISS drops the claim UNLESS the pointer is still
/// inside [`HOVER_RELEASE_PX`] of the anchor — and a held miss deliberately does NOT move the
/// anchor, so continued travel in one direction always escapes the band (a re-anchoring hold would
/// let a slow drag carry the claim across the whole map).
#[must_use]
pub(crate) fn hover_next(prev: HoverState, hit: bool, px: f64, py: f64, now_ms: f64) -> HoverState {
    if hit {
        return HoverState {
            last_ms: now_ms,
            pickable: true,
            anchor: Some((px, py)),
        };
    }
    let held = prev.pickable
        && prev
            .anchor
            .is_some_and(|(ax, ay)| (px - ax).hypot(py - ay) <= HOVER_RELEASE_PX);
    HoverState {
        last_ms: now_ms,
        pickable: held,
        anchor: if held { prev.anchor } else { None },
    }
}

/// The CSS cursor value for a hover verdict.
#[must_use]
pub(crate) fn hover_cursor_css(pickable: bool) -> &'static str {
    if pickable {
        HOVER_CURSOR_PICKABLE
    } else {
        HOVER_CURSOR_PLAIN
    }
}

/// Is the hover read suppressed right now? Pure, so the suppression set is a readable list rather
/// than a chain of early returns nobody can enumerate.
///
///   * `gesture_active` — a drag, marquee, rotate or measuring capture is in flight. The pointer is
///     committed to a gesture; re-labelling it mid-drag would be noise, and the cursor must not be
///     left claiming "pickable" over whatever the drag happens to be passing over.
///   * `place_armed` — a palette place, or a multi-click zone draw, owns the pointer, and the live
///     affordance is the place ghost rather than the cursor.
///   * `measuring` — the Ruler and line-of-sight tools are capturing points; the map's pickable
///     entities are not the subject.
///
/// `place_armed` and the zone draw are ALSO caught by the pending-gesture early return further up
/// the handler. They are named here anyway: the predicate is the statement of intent, and a later
/// edit that reorders the handler must not silently un-suppress them.
#[must_use]
pub(crate) fn hover_suppressed(gesture_active: bool, place_armed: bool, measuring: bool) -> bool {
    gesture_active || place_armed || measuring
}
