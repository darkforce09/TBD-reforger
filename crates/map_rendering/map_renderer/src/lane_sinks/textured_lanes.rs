//! **Role:** `TexturedLanes`, the engine's lanes borrowed apart from the rest of the engine and
//! lent to a world typed layer that writes textured lanes (terrain textures, the forest density,
//! the terrain line of sight overlay), as a `LaneSink` whose texture record is the world layers'
//! `TexLane`.
//! **Position:** the map renderer; `typed_layers/world_layers.rs` builds one beside the layer
//! fields it lends, so a layer can write lanes while it is itself borrowed from the engine.
//! **Signals & state:** the borrows of `UntexturedLanes`, whose texture record list it also
//! writes.
//! **Invariants:** the engine's own `LaneSink` rules (`engine_lane_sink.rs`) hold: the untextured
//! writes are `UntexturedLanes`'s, and a textured upsert replaces the lane's batch (dropping its
//! old record) before it keeps the new record beside it, so a record lives exactly as long as its
//! batch.

use crate::lane_sinks::untextured_lanes::UntexturedLanes;
use gpu_frame::frame::DrawBatch;
use render_primitives::frame::ids::LaneId;
use renderer_core::lane_sink::LaneSink;
use renderer_core::layer_context::LayerContext;
use world_layers_gpu::textured_lane::TexLane;

/// The engine's lane state and layer context borrowed apart from the rest of the engine, with
/// textured lanes allowed.
pub(crate) struct TexturedLanes<'a> {
    /// The borrowed lane state.
    pub(crate) lanes: UntexturedLanes<'a>,
}

impl LaneSink<TexLane> for TexturedLanes<'_> {
    fn upsert_lane_batch(&mut self, batch: DrawBatch) {
        self.lanes.upsert_lane_batch(batch);
    }

    fn remove_lane_batch(&mut self, lane: LaneId) {
        self.lanes.remove_lane_batch(lane);
    }

    fn upsert_textured_lane_batch(&mut self, batch: DrawBatch, texture: TexLane) {
        let lane = batch.lane;
        self.lanes.upsert_lane_batch(batch);
        self.lanes.tex_lanes.push((lane, texture));
    }

    fn lane_texture(&self, lane: LaneId) -> Option<&TexLane> {
        self.lanes
            .tex_lanes
            .iter()
            .find_map(|(l, t)| (*l == lane).then_some(t))
    }

    fn lane_batch(&self, lane: LaneId) -> Option<&DrawBatch> {
        self.lanes.lane_batch(lane)
    }

    fn lane_batch_mut(&mut self, lane: LaneId) -> Option<&mut DrawBatch> {
        self.lanes.lane_batch_mut(lane)
    }

    fn mark_damage(&mut self) {
        self.lanes.mark_damage();
    }

    fn layer_context(&mut self) -> LayerContext<'_> {
        self.lanes.layer_context()
    }
}
