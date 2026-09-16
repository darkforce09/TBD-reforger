//! Role: draw.
//! Position: `apps/website/graphics-engine/src` — geometry assembly and the draw path.
//! Signals & state: vertex/index streams and instance packing.
//! Invariants: takes geometry, returns geometry. It never asks what a shape represents.

/// Frustum compaction of packed sprite instances — CPU oracle plus its GPU compute twin.
pub mod cull;

/// Ear-clipping triangulation.
pub mod triangulate;
