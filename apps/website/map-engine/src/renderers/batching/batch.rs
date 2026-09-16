//! Role: batch.
//! Position: `renderers/batching` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::PipelineKind;
use crate::core::pipeline::draw_order::LaneRole;
use crate::renderers::primitives::hairlines::LineLane;
use crate::renderers::primitives::vector_lines::PolyLane;
use crate::terrain::satellite::textures::TexLane;

/// Batch payload.
pub(crate) enum BatchPayload {
    /// Instanced.
    Instanced { instances: wgpu::Buffer, count: u32 },

    /// Textured.
    Textured(TexLane),

    /// Lines.
    Lines(LineLane),

    /// world-building fill: `scene::BuildingInstance` stream (40 B), drawn `draw(0..4, 0..count)`.
    BuildingInstanced { instances: wgpu::Buffer, count: u32 },

    /// polygon fill / wide polyline strips (indexed triangle list).
    Polygon(PolyLane),

    /// atlas icon instances (`scene::IconInstance`, 20 B each).
    IconInstanced { instances: wgpu::Buffer, count: u32 },

    /// Marker composite.
    MarkerComposite {
        icons: wgpu::Buffer,
        icon_count: u32,
        captions: Option<(wgpu::Buffer, u32)>,
    },
}

impl BatchPayload {
    /// Kind.
    pub(crate) fn kind(&self) -> PipelineKind {
        match self {
            Self::Instanced { .. } => PipelineKind::QuadInstanced,
            Self::Textured(_) => PipelineKind::TexturedQuad,
            Self::Lines(_) => PipelineKind::Polyline,
            Self::BuildingInstanced { .. } => PipelineKind::BuildingQuad,
            Self::Polygon(_) => PipelineKind::PolygonFill,
            Self::IconInstanced { .. } | Self::MarkerComposite { .. } => {
                PipelineKind::IconInstanced
            }
        }
    }
}

/// Batch.
pub(crate) struct Batch {
    /// Role.
    pub(crate) role: LaneRole,

    /// Visible.
    pub(crate) visible: bool,

    /// Payload.
    pub(crate) payload: BatchPayload,
}

/// Indirect icon.
pub(crate) struct IndirectIcon<'a> {
    /// Role.
    pub(crate) role: LaneRole,

    /// Pipeline.
    pub(crate) pipeline: &'a wgpu::RenderPipeline,

    /// Atlas bind.
    pub(crate) atlas_bind: &'a wgpu::BindGroup,

    /// Instances.
    pub(crate) instances: &'a wgpu::Buffer,

    /// Indirect.
    pub(crate) indirect: &'a wgpu::Buffer,
}
