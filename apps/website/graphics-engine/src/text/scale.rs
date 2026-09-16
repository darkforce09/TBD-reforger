//! Role: text scale.
//! Position: `text` in the graphics engine.
//! Signals & state: the glyph-size anchor and the min-pixel clamp built on it.
//! Invariants: pure zoom arithmetic. Nothing here knows what the glyph spells.

/// Glyph size anchor: displayPx = baseSizePx * 2^(deckZoom − REF_ZOOM).
///
/// Moved here alone out of the caller's LOD table: every other constant there switches on a
/// world class name, while this one is the anchor every glyph size — moved and staying — is
/// measured against. The caller re-exports it at its former path.
pub const REF_ZOOM: f64 = 3.0;

/// Effective size with min-pixel clamp: `max(size_m, min_px · 2^−zoom)`.
///
/// Deliberately a **copy**, not a move: the caller keeps its own, and the two are four lines
/// of identical arithmetic. Moving it would have dragged a whole overlay module across the
/// wall just to reach it.
#[must_use]
pub fn size_with_min_px(size_m: f64, min_px: f64, deck_zoom: f64) -> f64 {
    let floor = min_px * 2.0_f64.powf(-deck_zoom);
    if size_m > floor { size_m } else { floor }
}
