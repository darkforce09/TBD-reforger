//! Role: glyph rasterisation and packing: the bitmap font, its baked atlas, glyph metrics, the
//! layout of a placed string into glyph instances, and the bit-packing of sprite instances.
//! Position: `text` in `render_primitives`; the crate's one home for glyph packing (`pack`) and
//! glyph sizing (`scale`). The graphics engine uploads the atlas and its uniform.
//! Signals & state: none; constant tables and pure functions.
//! Invariants: glyph shapes and byte layouts only. Which glyphs to draw, and where, is decided
//! by the caller.

/// Baked ASCII atlas.
pub mod atlas;

/// Bitmap font tables.
pub mod font;

/// Laying a placed string out into glyph instances.
pub mod layout;

/// Cell size in world meters, and the character → cell map.
pub mod metrics;

/// Bit-packing for sprite instances.
pub mod pack;

/// The glyph-size anchor and its min-pixel clamp.
pub mod scale;
