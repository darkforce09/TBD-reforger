//! **Role:** the render engine's side of a lane: `renderer_core`'s `LaneSink` over the engine's
//! persistent batch list, texture records and damage flag.
//! **Position:** the map renderer; the lane-role adapters in `lifecycle.rs` and the typed layers
//! write lanes through it, keyed by the opaque `LaneId`.
//! **Signals & state:** `RenderEngine::batches`, `RenderEngine::tex_lanes`, `RenderEngine::damage`
//! and the render statistics the layer context lends.
//! **Invariants:** the batch list is the frame packet's backing store and is only ever inserted
//! into, replaced or removed from here (ordered by lane through `gpu_frame::frame::packet`), never
//! rebuilt; every change that alters what is drawn marks damage, and removing an empty lane does
//! not; a lane's texture record lives exactly as long as its batch.

use crate::engine::RenderEngine;
use gpu_frame::frame::{DrawBatch, packet};
use render_primitives::frame::ids::LaneId;
use renderer_core::lane_sink::LaneSink;
use renderer_core::layer_context::LayerContext;
use world_layers_gpu::textured_lane::TexLane;

impl LaneSink<TexLane> for RenderEngine {
    fn upsert_lane_batch(&mut self, batch: DrawBatch) {
        let lane = batch.lane;
        self.tex_lanes.retain(|(l, _)| *l != lane);
        packet::upsert(&mut self.batches, batch);
        self.damage.mark();
    }

    fn remove_lane_batch(&mut self, lane: LaneId) {
        self.tex_lanes.retain(|(l, _)| *l != lane);
        if packet::remove(&mut self.batches, lane) {
            self.damage.mark();
        }
    }

    fn upsert_textured_lane_batch(&mut self, batch: DrawBatch, texture: TexLane) {
        let lane = batch.lane;
        self.upsert_lane_batch(batch);
        self.tex_lanes.push((lane, texture));
    }

    fn lane_texture(&self, lane: LaneId) -> Option<&TexLane> {
        self.tex_lanes
            .iter()
            .find_map(|(l, t)| (*l == lane).then_some(t))
    }

    fn lane_batch(&self, lane: LaneId) -> Option<&DrawBatch> {
        self.batches.iter().find(|b| b.lane == lane)
    }

    fn lane_batch_mut(&mut self, lane: LaneId) -> Option<&mut DrawBatch> {
        self.batches.iter_mut().find(|b| b.lane == lane)
    }

    fn mark_damage(&mut self) {
        self.mark_dirty();
    }

    fn layer_context(&mut self) -> LayerContext<'_> {
        LayerContext::new(
            self.gpu.device(),
            self.gpu.queue(),
            self.gpu.surface_format(),
            &self.frame_pipelines,
            &self.frame_bind_groups,
            &mut self.render_stats,
        )
    }
}
