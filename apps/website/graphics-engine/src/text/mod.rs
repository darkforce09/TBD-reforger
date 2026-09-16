//! Role: text.
//! Position: `apps/website/graphics-engine/src` — glyph rasterisation and packing.
//! Signals & state: a bitmap font, its baked atlas, and the bit-packing for sprite instances.
//! Invariants: glyph shapes and byte layouts only. Which glyphs to draw, and where, is decided
//! by the caller and arrives as a `frame::TextRun`.

/// Baked ASCII atlas.
pub mod atlas;

/// Bitmap font tables.
pub mod font;

/// Bit-packing for sprite instances.
pub mod pack;
