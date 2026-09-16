//! Role: lanes.
//! Position: `renderers/batching` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::scene::ANCHOR;
use website_graphics_engine::draw::geometry;
use website_graphics_engine::draw::grid;

/// Re-export `website_graphics_engine::draw::geometry::LineVertex`.
// T-0xx Phase 1D: the vertex layout and the placement arithmetic moved to
// `website-graphics-engine`. What could NOT cross is [`ANCHOR`] — the Everon terrain centre —
// so `rel`, `world_rect_rel` and `grid_lines` take it as an argument over there, and the two
// wrappers below bind it here. That is the whole shape of the split in four lines: the
// renderer does the arithmetic, this crate supplies the world fact.
pub use website_graphics_engine::draw::geometry::LineVertex;

/// Re-export `website_graphics_engine::draw::geometry::corner_uv`.
pub use website_graphics_engine::draw::geometry::corner_uv;

/// Re-export `website_graphics_engine::draw::geometry::pack_offset`.
pub use website_graphics_engine::draw::geometry::pack_offset;

/// Build the procedural 1 km grid as a `LineList` vertex buffer, anchored at [`ANCHOR`].
#[must_use]
pub fn grid_lines(width: f64, height: f64, over_hillshade: bool) -> Vec<LineVertex> {
    grid::grid_lines(ANCHOR, width, height, over_hillshade)
}

/// Anchor-relative-meters `[minX, minY, maxX, maxY]` (f32) for a world rect — the textured-quad instance geometry, matching the `scene::QuadInstance` anchor contract.
#[must_use]
pub fn world_rect_rel(min: [f64; 2], max: [f64; 2]) -> [f32; 4] {
    geometry::world_rect_rel(ANCHOR, min, max)
}
