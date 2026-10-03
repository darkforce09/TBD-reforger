//! The fixed pipeline and bind-group ids a frame packet's tables are indexed by.
//!
//! **Role:** names the nine slots of a frame packet's pipeline table ([`PIPELINE_SLOTS`]) and the
//! five fixed slots of its bind-group table, ahead of one texture slot per lane
//! ([`tex_bind_id`]).
//! **Position:** the renderer fills `FramePacket::pipelines` and `FramePacket::bind_groups` in
//! this order each frame; a layer stamps these ids on the `DrawBatch` it hands the lane sink, and
//! `gpu_frame`'s encoder binds by them without asking what a lane is.
//! **Signals & state:** none; constants and one pure function.
//! **Invariants:** the pipeline ids are dense, `0..PIPELINE_SLOTS`; the fixed bind ids are dense
//! below [`BIND_TEX_BASE`]; a lane's texture slot is `BIND_TEX_BASE + lane`, a pure function of its
//! lane, so the slot is stable for the life of the lane with no allocator and no free list. How
//! many texture slots the table holds is the caller's: it knows its widest lane id.

use render_primitives::frame::ids::{BindGroupId, LaneId, PipelineId};

/// `vs_quad`: axis-aligned coloured quads.
pub const PIPE_QUAD: PipelineId = PipelineId(0);

/// `vs_textured`: one sampled rect.
pub const PIPE_TEXTURED: PipelineId = PipelineId(1);

/// The density variant of the textured pipeline: one sampled density raster tinted at draw time.
pub const PIPE_DENSITY: PipelineId = PipelineId(2);

/// `LineList`.
pub const PIPE_LINE: PipelineId = PipelineId(3);

/// Oriented quads carrying their own basis (40-byte instances).
pub const PIPE_ORIENTED_QUAD: PipelineId = PipelineId(4);

/// Indexed triangle fills.
pub const PIPE_POLYGON: PipelineId = PipelineId(5);

/// Atlas-sampled sprites.
pub const PIPE_ICON: PipelineId = PipelineId(6);

/// Atlas-sampled glyphs.
pub const PIPE_TEXT: PipelineId = PipelineId(7);

/// The storage-buffer sprite pipeline the compute-culled indirect draws bind.
pub const PIPE_ICON_STORAGE32: PipelineId = PipelineId(8);

/// How many pipeline slots a frame packet carries.
pub const PIPELINE_SLOTS: usize = 9;

/// Group 0 for every draw: the camera uniform.
pub const BIND_CAMERA: BindGroupId = BindGroupId(0);

/// The glyph cell atlas.
pub const BIND_GLYPH_ATLAS: BindGroupId = BindGroupId(1);

/// The text cell atlas.
pub const BIND_TEXT_ATLAS: BindGroupId = BindGroupId(2);

/// The movable-sprite atlas at rest: its drag offset uniform reads zero.
pub const BIND_MOVABLE_SPRITE_ATLAS: BindGroupId = BindGroupId(3);

/// The movable-sprite atlas with the drag offset uniform applied.
pub const BIND_MOVABLE_SPRITE_ATLAS_DRAGGED: BindGroupId = BindGroupId(4);

/// The first bind-group slot reserved for a textured lane's own texture.
pub const BIND_TEX_BASE: u16 = 5;

/// The bind-group slot holding `lane`'s own texture: [`BIND_TEX_BASE`] plus the lane id.
#[must_use]
pub fn tex_bind_id(lane: LaneId) -> BindGroupId {
    BindGroupId(BIND_TEX_BASE + lane.0)
}

#[cfg(test)]
#[path = "tests/packet_bindings_tests.rs"]
mod tests;
