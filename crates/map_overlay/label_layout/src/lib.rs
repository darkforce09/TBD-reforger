//! Map label layout: which labels draw at a zoom, and their glyph instances.
//!
//! **Role:** declutters generic map labels by distance and importance ([`declutter`]), decides
//! which towns draw and how they fade ([`importance`]), sizes and keys the world's object
//! glyphs ([`glyph_math`]), and packs decluttered labels into monospaced glyph instances
//! ([`text_packing`]); [`label_ids`] holds the label and location ids.
//! **Position:** map overlay tier 1, over `render_primitives` for the glyph layout and packing
//! and `newtype_ids` for the ids. The map engine's place-name packers, location loader, draw
//! buffers and marker captions read it.
//! **Signals & state:** none; constants, plain records and pure functions.
//! **Invariants:** a label's importance decides only whether it draws and never reaches the
//! renderer; no module names a particular peak, town or road.

pub mod declutter;
pub mod glyph_math;
pub mod importance;
pub mod label_ids;
pub mod prelude;
pub mod text_packing;
