//! Role: Module boundary for renderers/text/layout.
//! Position: `renderers/text/layout` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::renderers::text::atlas::TEXT_ATLAS_COLS`.
pub use crate::renderers::text::atlas::TEXT_ATLAS_COLS;

/// Re-export `crate::renderers::text::atlas::TEXT_ATLAS_ROWS`.
pub use crate::renderers::text::atlas::TEXT_ATLAS_ROWS;

/// Re-export `crate::renderers::text::atlas::TEXT_CELL_PX`.
pub use crate::renderers::text::atlas::TEXT_CELL_PX;

/// Re-export `crate::renderers::text::atlas::TEXT_HALO_PX`.
pub use crate::renderers::text::atlas::TEXT_HALO_PX;

/// Re-export `crate::renderers::text::atlas::TEXT_HALO_RGBA`.
pub use crate::renderers::text::atlas::TEXT_HALO_RGBA;

/// Re-export `crate::renderers::text::atlas::TEXT_INK_RGBA`.
pub use crate::renderers::text::atlas::TEXT_INK_RGBA;

/// Re-export `crate::renderers::text::atlas::TOFU_GLYPH`.
pub use crate::renderers::text::atlas::TOFU_GLYPH;

/// Re-export `crate::renderers::text::atlas::bake_ascii_atlas_rgba`.
pub use crate::renderers::text::atlas::bake_ascii_atlas_rgba;

/// Re-export `crate::renderers::text::atlas::glyph_cell_uv`.
pub use crate::renderers::text::atlas::glyph_cell_uv;

/// Re-export `crate::renderers::text::metrics::TEXT_GLYPH_ADVANCE_RATIO`.
pub use crate::renderers::text::metrics::TEXT_GLYPH_ADVANCE_RATIO;

/// Re-export `crate::renderers::text::metrics::TextGlyphInstance`.
pub use crate::renderers::text::metrics::TextGlyphInstance;

/// Re-export `crate::renderers::text::metrics::glyph_index_for_char`.
pub use crate::renderers::text::metrics::glyph_index_for_char;

/// Re-export `crate::renderers::text::metrics::height_label_sep_m`.
pub use crate::renderers::text::metrics::height_label_sep_m;

/// Re-export `crate::renderers::text::metrics::text_char_meters`.
pub use crate::renderers::text::metrics::text_char_meters;

/// Re-export `crate::renderers::text::packing::pack_height_label_glyphs`.
pub use crate::renderers::text::packing::pack_height_label_glyphs;

/// Re-export `crate::renderers::text::packing::pack_label_glyphs`.
pub use crate::renderers::text::packing::pack_label_glyphs;

/// Re-export `crate::renderers::text::packing::pack_road_label_bytes`.
pub use crate::renderers::text::packing::pack_road_label_bytes;

/// Re-export `crate::renderers::text::packing::pack_text_icon_bytes`.
pub use crate::renderers::text::packing::pack_text_icon_bytes;

/// Re-export `crate::renderers::text::packing::pack_text_icon_bytes_tint`.
pub use crate::renderers::text::packing::pack_text_icon_bytes_tint;

/// Re-export `crate::renderers::text::packing::pack_town_label_bytes`.
pub use crate::renderers::text::packing::pack_town_label_bytes;

/// Re-export `crate::renderers::text::packing::pack_town_label_glyphs`.
pub use crate::renderers::text::packing::pack_town_label_glyphs;
#[cfg(test)]
mod tests;
