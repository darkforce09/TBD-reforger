//! Role: CPU geometry for the draw path: instance layouts, the line vertex and its placement,
//! triangulation, fill and hairline composition, the procedural grid and the sprite-cull
//! reference.
//! Position: `draw` in `render_primitives`; the graphics engine uploads what it produces.
//! Signals & state: none; layouts and pure functions.
//! Invariants: takes geometry, returns geometry. It never asks what a shape represents.

/// Triangulated fills and hairline segment lists.
pub mod compose;

/// The CPU reference of sprite frustum culling.
pub mod cull;

/// One line vertex, and placing points and rects against the caller's anchor.
pub mod geometry;

/// The procedural grid.
pub mod grid;

/// Per-instance vertex layouts.
pub mod instances;

/// Ear-clipping triangulation.
pub mod triangulate;
