//! Role: glyph metrics — cell size in world meters, and the character → cell map.
//! Position: `overlay/symbology` in the map engine.
//! Signals & state: the atlas cell grid and the advance ratio labels are laid out against.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.
//!
//! T-0xx Phase 2B.1: from `renderers/text/metrics.rs`. The baked ASCII atlas and the bitmap
//! font are re-exported below at the paths `renderers/text/mod.rs` used to publish, because
//! their twenty-one consumers are label belts that sit next to this file, not next to a GPU.

/// The baked ASCII atlas — cell grid, halo/ink colours, and the RGBA bake itself.
pub use website_graphics_engine::text::atlas;

/// The 16×32 bitmap font tables the atlas is baked from.
pub use website_graphics_engine::text::font;

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

#[cfg(test)]
#[path = "tests/text_layout.rs"]
mod text_layout_tests;
