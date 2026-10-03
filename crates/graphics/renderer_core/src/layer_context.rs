//! The GPU context a renderer lends a layer while the layer builds or updates its resources.
//!
//! **Role:** [`LayerContext`] borrows the renderer's device, queue and surface format, the
//! pipeline and bind-group tables a frame packet is indexed by, and the renderer's
//! [`RenderStats`], so a layer can create buffers and bind groups, look up the pipeline or atlas
//! a batch names, and report its lane counts without holding the renderer.
//! **Position:** handed out by `crate::lane_sink::LaneSink::layer_context`; a typed layer reads it
//! while it builds the batch it then upserts through the lane sink.
//! **Signals & state:** none of its own; every field is a borrow of the renderer's state, the
//! statistics mutably.
//! **Invariants:** the pipeline table is indexed by [`PipelineId`] and the bind-group table by
//! [`BindGroupId`], as [`crate::packet_bindings`] numbers them; a slot the renderer has not filled
//! (an atlas whose upload has not landed) reads `None`.

use crate::render_stats::RenderStats;
use render_primitives::frame::ids::{BindGroupId, PipelineId};

/// A borrowed view of the renderer's GPU state and statistics.
pub struct LayerContext<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    surface_format: wgpu::TextureFormat,
    pipelines: &'a [wgpu::RenderPipeline],
    bind_groups: &'a [Option<wgpu::BindGroup>],
    stats: &'a mut RenderStats,
}

impl<'a> LayerContext<'a> {
    /// Borrow the renderer's device, queue, surface format, packet tables and statistics.
    #[must_use]
    pub fn new(
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        pipelines: &'a [wgpu::RenderPipeline],
        bind_groups: &'a [Option<wgpu::BindGroup>],
        stats: &'a mut RenderStats,
    ) -> Self {
        Self {
            device,
            queue,
            surface_format,
            pipelines,
            bind_groups,
            stats,
        }
    }

    /// The renderer's device.
    #[must_use]
    pub fn device(&self) -> &'a wgpu::Device {
        self.device
    }

    /// The renderer's queue.
    #[must_use]
    pub fn queue(&self) -> &'a wgpu::Queue {
        self.queue
    }

    /// The format of the surface the renderer presents to.
    #[must_use]
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_format
    }

    /// The pipeline in slot `id` of the packet's pipeline table, if the table holds one there.
    #[must_use]
    pub fn pipeline(&self, id: PipelineId) -> Option<&'a wgpu::RenderPipeline> {
        self.pipelines.get(usize::from(id.0))
    }

    /// The bind group in slot `id` of the packet's bind-group table, if one is built there.
    #[must_use]
    pub fn bind_group(&self, id: BindGroupId) -> Option<&'a wgpu::BindGroup> {
        self.bind_groups
            .get(usize::from(id.0))
            .and_then(Option::as_ref)
    }

    /// The renderer's statistics.
    #[must_use]
    pub fn stats(&self) -> &RenderStats {
        self.stats
    }

    /// The renderer's statistics, for a layer to report its lane counts into.
    pub fn stats_mut(&mut self) -> &mut RenderStats {
        self.stats
    }
}
