//! The plain paste's anchor decision: the map cursor, else the view centre, else no paste.

/// **The off-map plain paste falls back to the view centre, never to "no anchor".**
///
/// The keydown arm itself cannot run off-browser (it reads a live camera), so the DECISION is
/// factored into a pure function and tested here. A `None` anchor for a missing cursor would
/// hand `paste_at_cursor` no anchor and turn a plain paste into a paste-at-original.
#[test]
fn t743_plain_paste_falls_back_to_the_view_centre() {
    // Cursor on the map wins outright — the fallback must not override a real cursor.
    assert_eq!(
        super::plain_paste_anchor(Some((10.0, 20.0)), Some((999.0, 999.0))),
        Some((10.0, 20.0)),
        "a live map cursor is the anchor; the view centre is only a fallback"
    );
    // Pointer over a chrome panel (the common case) — anchored on the view centre, and NOT
    // silently promoted to paste-at-original.
    assert_eq!(
        super::plain_paste_anchor(None, Some((640.0, 480.0))),
        Some((640.0, 480.0)),
        "an off-map plain paste must still carry an anchor — the middle of what is on screen"
    );
    // No camera at all (engine not booted, or a singular matrix) — there is nothing to anchor
    // on and nothing on screen, so the keypress does not paste.
    assert_eq!(
        super::plain_paste_anchor(None, None),
        None,
        "with no cursor and no camera the plain paste must decline, not invent a coordinate"
    );
}
