//! Role: frame batch.
//! Position: `frame` in the graphics engine.
//! Signals & state: one draw's worth of GPU handles.
//! Invariants: variants are named for their VERTEX LAYOUT, never for their subject. Two batches
//! that share a layout share a variant however different the things they depict.

use crate::frame::buffers::{IndexedMesh, InstanceBuffer, VertexStream};
use crate::frame::ids::{BindGroupId, LaneId, PipelineId};
use crate::frame::text::TextRun;

/// What a batch draws, by vertex layout.
pub enum DrawPayload {
    /// Axis-aligned coloured quads (32 B: `min`, `max`, `color`).
    Quads(InstanceBuffer),

    /// Oriented quads carrying their own basis (40 B: `center`, `half`, `basis`, `color`).
    OrientedQuads(InstanceBuffer),

    /// Atlas-sampled sprites (20 B: `pos`, `size`, `yaw`, `glyph`, `tint`).
    Sprites {
        /// Packed sprite instances.
        instances: InstanceBuffer,
        /// Atlas to sample.
        atlas: BindGroupId,
    },

    /// One textured rect plus the bind group holding its texture.
    ///
    /// The caller keeps its own texture lifetime and tiling bookkeeping; the renderer gets a
    /// buffer and a bind group.
    TexturedRect {
        /// The rect's instance data.
        instances: InstanceBuffer,
        /// Bind group holding the texture and sampler.
        texture: BindGroupId,
    },

    /// `LineList` vertex stream.
    Lines(VertexStream),

    /// Indexed triangle list.
    Indexed(IndexedMesh),

    /// Sprites with an optional glyph run drawn in the same lane.
    SpritesWithText {
        /// Packed sprite instances.
        sprites: InstanceBuffer,
        /// Atlas the sprites sample.
        atlas: BindGroupId,
        /// Optional co-drawn glyphs.
        text: Option<TextRun>,
    },

    /// A glyph run occupying a lane of its own.
    Text(TextRun),
}

/// One draw, keyed by an opaque lane and carrying its own pipeline.
///
/// `pipeline` is explicit for a reason: it is what allows the encoder to bind without ever
/// asking what the lane *is*. Every decision that used to be a match on lane identity — which
/// pipeline, which bind group — is made by the caller and travels in this struct.
pub struct DrawBatch {
    /// Draw-order key. The packet is sorted ascending on it.
    pub lane: LaneId,

    /// Skip this batch without disturbing the lane's buffers.
    pub visible: bool,

    /// Pipeline to bind.
    pub pipeline: PipelineId,

    /// Geometry and its bind groups.
    pub payload: DrawPayload,
}

/// An indirect draw: the instance count lives in a GPU buffer the compute pass wrote.
pub struct IndirectDraw<'a> {
    /// Draw-order key.
    pub lane: LaneId,

    /// Pipeline to bind.
    pub pipeline: PipelineId,

    /// Atlas the sprites sample.
    pub atlas: BindGroupId,

    /// Packed sprite instances.
    pub instances: &'a wgpu::Buffer,

    /// Buffer holding the `draw_indirect` arguments.
    pub indirect: &'a wgpu::Buffer,
}
