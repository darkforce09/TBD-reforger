//! Role: frame packet.
//! Position: `frame` in the graphics engine.
//! Signals & state: one frame's complete draw list.
//! Invariants: ordering is the CALLER's policy. The renderer merges by ascending lane and
//! asserts sortedness in debug; it never re-ranks.

use crate::frame::batch::{DrawBatch, IndirectDraw};
use crate::frame::camera::CameraUniform;
use crate::frame::ids::BindGroupId;
use crate::frame::text::TextRun;

/// Everything one frame needs.
///
/// `text` is merged with `batches` by ascending lane rather than concatenated. Concatenation
/// would draw every glyph run after every batch, which reorders lanes against each other and
/// changes the picture — so the merge is load-bearing, not tidiness.
pub struct FramePacket<'a> {
    /// Group 0 binding 0 for every draw.
    pub camera: CameraUniform,

    /// Colour the pass clears to.
    pub clear: wgpu::Color,

    /// Ascending by `lane`.
    pub batches: &'a [DrawBatch],

    /// Glyph runs the caller chose to address separately. Merged with `batches` by lane.
    pub text: &'a [TextRun],

    /// Indirect draws, merged by lane like the rest.
    pub indirect: &'a [IndirectDraw<'a>],

    /// Pipelines addressed by [`crate::frame::ids::PipelineId`].
    pub pipelines: &'a [wgpu::RenderPipeline],

    /// Bind groups addressed by [`crate::frame::ids::BindGroupId`].
    pub bind_groups: &'a [wgpu::BindGroup],

    /// The camera bind group, bound at group 0 for every draw.
    pub camera_bind: BindGroupId,

    /// Vertex stream 0 for every instanced draw.
    pub unit_quad: &'a wgpu::Buffer,
}

impl<'a> FramePacket<'a> {
    /// Debug-only proof that the caller ordered what it promised to order.
    ///
    /// Not an assert in release: a mis-sorted packet draws in the wrong order, which is a
    /// visible bug, but it is not unsound and must not abort a shipping frame.
    #[must_use]
    pub fn batches_sorted(&self) -> bool {
        self.batches.windows(2).all(|w| w[0].lane <= w[1].lane)
    }
}
