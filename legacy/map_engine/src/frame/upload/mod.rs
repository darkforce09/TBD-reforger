//! Role: Module boundary for frame/upload.
//! Position: `frame/upload` in the map engine.
//! Signals & state: the belts that turn already-computed geometry into a lane's GPU buffer
//! and the `DrawBatch` that points at it.
//! Invariants: an upload belt does not decide anything. What to draw, in which lane, at what
//! zoom, in what colour — all of that is settled before a belt is called.
//!
//! T-0xx Phase 2B.1: assembled from `renderers/primitives/{selection,vector_lines,hairlines}.rs`
//! and `renderers/text/lanes.rs`. `vector_lines.rs` is `polygons.rs` here because that is what
//! it uploads — indexed triangle meshes and line strips for the vector lanes.

/// Hairline segment lanes.
pub mod hairlines;

/// Polygon-mesh and line-strip lanes.
pub mod polygons;

/// The selection marquee and its outline.
pub mod selection;

/// Cartographic, town and road label lanes.
pub mod text;
