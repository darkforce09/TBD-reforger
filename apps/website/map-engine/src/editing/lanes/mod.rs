//! Role: Module boundary for the editor-authored overlay lanes the map draws and picks.
//! Position: `editing::lanes` in the map engine.
//! Signals & state: none of its own — a lane is a deterministic function of the document JSON and
//! the world coordinates handed to it.
//! Invariants: one document read feeds both the vertices a lane uploads and the hit test that
//! picks them, so what is drawn and what a click can find are the same set by construction; every
//! lane's coordinates are world metres on the map plane.

/// The connection graph's hairlines: the drawable edges, their packed vertices, and the edge
/// under a click.
pub mod connections;

/// The editor-only annotation glyphs: the document's comment points, the lanes packed from them
/// (at rest and mid-drag), and the glyph under a click.
pub mod comments;

/// The briefing markers' four row-aligned lane columns.
pub mod markers;
