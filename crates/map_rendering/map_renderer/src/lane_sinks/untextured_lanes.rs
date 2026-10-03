//! **Role:** `UntexturedLanes`, the engine's lanes borrowed apart from the rest of the engine and
//! lent to a typed layer that writes untextured lanes only (the symbology's sprite layers), as a
//! `LaneSink` whose texture record type is `Infallible`.
//! **Position:** the map renderer; `typed_layers/symbology_layers.rs` builds one beside the layer
//! fields it lends, so a layer can write lanes while it is itself borrowed from the engine.
//! **Signals & state:** borrows of `RenderEngine::batches`, `RenderEngine::tex_lanes`,
//! `RenderEngine::damage`, the layer context's parts and the render statistics.
//! **Invariants:** the batch list writes keep the engine's own `LaneSink` rules
//! (`engine_lane_sink.rs`): ordered upsert and removal through `gpu_frame::frame::packet`, a
//! lane's texture record dropped with its batch, damage marked on every change that alters what
//! is drawn and not on removing an empty lane; no textured lane can be written or read through it.

use gpu_frame::frame::{DrawBatch, packet};
use render_primitives::frame::damage::RenderDamage;
use render_primitives::frame::ids::LaneId;
use renderer_core::lane_sink::LaneSink;
use renderer_core::layer_context::LayerContext;
use renderer_core::render_stats::RenderStats;
use std::convert::Infallible;
use world_layers_gpu::textured_lane::TexLane;

/// The engine's lane state and layer context borrowed apart from the rest of the engine.
pub(crate) struct UntexturedLanes<'a> {
    /// The persistent batch list.
    pub(crate) batches: &'a mut Vec<DrawBatch>,

    /// The texture records of the live textured lanes.
    pub(crate) tex_lanes: &'a mut Vec<(LaneId, TexLane)>,

    /// The damage flag.
    pub(crate) damage: &'a mut RenderDamage,

    /// The device.
    pub(crate) device: &'a wgpu::Device,

    /// The queue.
    pub(crate) queue: &'a wgpu::Queue,

    /// The surface format.
    pub(crate) surface_format: wgpu::TextureFormat,

    /// The frame packet's pipeline table.
    pub(crate) frame_pipelines: &'a [wgpu::RenderPipeline],

    /// The frame packet's bind-group table.
    pub(crate) frame_bind_groups: &'a [Option<wgpu::BindGroup>],

    /// The render statistics.
    pub(crate) render_stats: &'a mut RenderStats,
}

impl LaneSink<Infallible> for UntexturedLanes<'_> {
    fn upsert_lane_batch(&mut self, batch: DrawBatch) {
        let lane = batch.lane;
        self.tex_lanes.retain(|(l, _)| *l != lane);
        packet::upsert(self.batches, batch);
        self.damage.mark();
    }

    fn remove_lane_batch(&mut self, lane: LaneId) {
        self.tex_lanes.retain(|(l, _)| *l != lane);
        if packet::remove(self.batches, lane) {
            self.damage.mark();
        }
    }

    fn upsert_textured_lane_batch(&mut self, _batch: DrawBatch, texture: Infallible) {
        match texture {}
    }

    fn lane_texture(&self, _lane: LaneId) -> Option<&Infallible> {
        None
    }

    fn lane_batch(&self, lane: LaneId) -> Option<&DrawBatch> {
        self.batches.iter().find(|b| b.lane == lane)
    }

    fn lane_batch_mut(&mut self, lane: LaneId) -> Option<&mut DrawBatch> {
        self.batches.iter_mut().find(|b| b.lane == lane)
    }

    fn mark_damage(&mut self) {
        self.damage.mark();
    }

    fn layer_context(&mut self) -> LayerContext<'_> {
        LayerContext::new(
            self.device,
            self.queue,
            self.surface_format,
            self.frame_pipelines,
            self.frame_bind_groups,
            self.render_stats,
        )
    }
}
