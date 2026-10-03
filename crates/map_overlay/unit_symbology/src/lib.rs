//! The unit symbology of the map: what a slot, a vehicle, a marker and a squad draw as.
//!
//! **Role:** the side tints and the role and vehicle class tables ([`classification`]), the
//! symbol atlas cells ([`symbol_atlas`]), the briefing marker vocabulary, atlas and captions
//! ([`markers`]) and the squad link hairlines ([`squad_links`]).
//! **Position:** map overlay tier 2, over `map_draw_lanes` (caption sizing), `render_primitives`
//! (text layout) and `orbat_slot_ids` (the slot ids the squad links name). `overlay_instances`
//! packs instances against these cells and tints; the map engine's slot, marker and editing
//! lanes read it.
//! **Signals & state:** none; constants, tables and pure functions into owned buffers.
//! **Invariants:** this is the one role → glyph and alias → silhouette table in the tree; the
//! cells and aliases are ORBAT and mission vocabulary, so the renderer never sees them.

pub mod classification;
pub mod markers;
pub mod prelude;
pub mod squad_links;
pub mod symbol_atlas;
