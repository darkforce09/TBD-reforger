//! Role: the height-label separation oracle the label glyph tests measure against.
//! Position: `overlay/symbology` in the map engine; the glyph cell metrics, the baked ASCII
//! atlas and the bitmap font it is tested beside live in `render_primitives::text`.
//! Signals & state: none; pure functions.
//! Invariants: the separation is a cartographic rule about peaks, never a glyph-cell property.

use crate::world::environment::locations::peaks::height_label_min_sep_m;

/// G4 oracle for height labels (re-export for tests).
///
/// Stays: the minimum separation between two HEIGHT labels is a cartographic rule about peaks,
/// not a property of a glyph cell.
#[must_use]
pub fn height_label_sep_m(deck_zoom: f64) -> f64 {
    height_label_min_sep_m(deck_zoom)
}

#[cfg(test)]
#[path = "tests/text_layout.rs"]
mod text_layout_tests;
