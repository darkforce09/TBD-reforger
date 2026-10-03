//! **Role:** the geometry and pipeline rules of a one-quad textured lane: the anchor-relative
//! rectangle the quad covers and the pipeline the lane draws with.
//! **Position:** used by `textured_lane`'s quad lane upsert and by the forest, terrain texture and
//! terrain line of sight overlay layers when they build a quad instance.
//! **Signals & state:** none; pure functions.
//! **Invariants:** quad geometry is measured relative to
//! `map_coordinates::terrain_frames::ANCHOR`; the forest fill samples its density raster with the
//! density pipeline and every other textured lane draws with the plain textured pipeline.

use map_coordinates::terrain_frames::ANCHOR;
use map_draw_lanes::lane_roles::LaneRole;
use render_primitives::draw::geometry;
use render_primitives::frame::ids::PipelineId;
use renderer_core::packet_bindings::{PIPE_DENSITY, PIPE_TEXTURED};

/// Anchor-relative metres `[minX, minY, maxX, maxY]` (f32) for a world rectangle: the textured
/// quad's instance geometry, matching the `QuadInstance` anchor contract.
// The arithmetic is `render_primitives::draw::geometry::world_rect_rel`; this wrapper binds it
// to [`ANCHOR`], a fact about a specific 12.8 km world, and nothing else.
#[must_use]
pub(crate) fn world_rect_rel(min: [f64; 2], max: [f64; 2]) -> [f32; 4] {
    geometry::world_rect_rel(ANCHOR, min, max)
}

/// Which pipeline a textured lane draws with: the forest fill samples a density raster, every
/// other textured lane a plain texture.
#[must_use]
pub(crate) fn textured_pipeline_for(role: LaneRole) -> PipelineId {
    if role == LaneRole::ForestFill {
        PIPE_DENSITY
    } else {
        PIPE_TEXTURED
    }
}

#[cfg(test)]
#[path = "tests/textured_quad_tests.rs"]
mod tests;
