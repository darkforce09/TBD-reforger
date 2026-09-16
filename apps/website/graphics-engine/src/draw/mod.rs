//! Role: draw.
//! Position: `apps/website/graphics-engine/src` — geometry assembly and the draw path.
//! Signals & state: vertex/index streams and instance packing.
//! Invariants: takes geometry, returns geometry. It never asks what a shape represents.

/// Triangulated fills and hairline segment lists.
pub mod compose;

/// Frustum compaction of packed sprite instances — CPU oracle plus its GPU compute twin.
pub mod cull;

/// One line vertex, and placing a rect against the caller's anchor.
pub mod geometry;

/// The procedural grid.
pub mod grid;

/// Per-instance vertex layouts.
pub mod instances;

/// Ear-clipping triangulation.
pub mod triangulate;
