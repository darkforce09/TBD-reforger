//! Role: metrics.
//! Position: `renderers/text` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::environment::locations::peaks::height_label_min_sep_m;

/// Re-export `website_graphics_engine::text::metrics::TEXT_GLYPH_ADVANCE_RATIO`.
// T-0xx Phase 1D: the cell metrics moved to `website-graphics-engine`. Re-exported at their
// former path so every call site in this crate keeps its spelling — the move is a relocation,
// not a rename.
pub use website_graphics_engine::text::metrics::TEXT_GLYPH_ADVANCE_RATIO;

/// Re-export `website_graphics_engine::text::metrics::TextGlyphInstance`.
pub use website_graphics_engine::text::metrics::TextGlyphInstance;

/// Re-export `website_graphics_engine::text::metrics::glyph_index_for_char`.
pub use website_graphics_engine::text::metrics::glyph_index_for_char;

/// Re-export `website_graphics_engine::text::metrics::text_char_meters`.
pub use website_graphics_engine::text::metrics::text_char_meters;

/// G4 oracle for height labels (re-export for tests).
///
/// Stays: the minimum separation between two HEIGHT labels is a cartographic rule about peaks,
/// not a property of a glyph cell.
#[must_use]
pub fn height_label_sep_m(deck_zoom: f64) -> f64 {
    height_label_min_sep_m(deck_zoom)
}
