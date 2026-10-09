//! The arithmetic that turns vector shapes into the samples of a regular grid.
//!
//! **Role:** rounding with ties toward +∞ ([`half_up_rounding`]), the uniform Catmull-Rom spline
//! with its plan-view tangent and normal ([`catmull_rom`]), the even-odd scanline fill of a polygon
//! over a sample grid ([`polygon_scanline`]) and the anti-aliased disc stamps that stroke a segment
//! on a pixel canvas ([`disc_stamping`]).
//! **Position:** geometry tier 0, with no dependency. The map raster pipeline's export image lanes
//! (`tools/map_assets/map_raster_pipeline`) rasterize the Workbench water and road exports with it.
//! **Signals & state:** none; pure functions and plain `Copy` values.
//! **Invariants:** every function is deterministic to the bit: each expression keeps one fixed
//! evaluation order with no fused multiply-add, so the same inputs give the same samples on every
//! target; nothing here reads a file, allocates a canvas or knows a map concept.

pub mod catmull_rom;
pub mod disc_stamping;
pub mod half_up_rounding;
pub mod polygon_scanline;
pub mod prelude;
