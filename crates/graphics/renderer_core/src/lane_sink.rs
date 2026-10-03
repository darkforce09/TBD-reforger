//! What a layer needs from the renderer to own a lane.
//!
//! **Role:** [`LaneSink`] is the renderer's side of a lane: insert, replace or remove a lane's
//! draw batch, keep the texture record a textured lane's batch cannot carry, read or adjust a
//! batch in place, mark the frame damaged, name a lane's texture slot, and lend the layer a
//! [`LayerContext`].
//! **Position:** the map renderer implements it over its persistent batch list; a typed layer
//! (sprites, geometry, textures) writes through `&mut dyn LaneSink<_>` instead of reaching
//! into the renderer's fields, and maps its own lane names onto the opaque [`LaneId`] before it
//! calls.
//! **Signals & state:** none of its own; an implementation owns the batch list, the texture
//! records and the damage flag.
//! **Invariants:** a lane holds at most one batch, keyed by `DrawBatch::lane`; upserting a lane
//! drops its texture record, so a textured lane's record lives exactly as long as its batch;
//! every change to the batch list that changes what is drawn marks damage, and removing a lane
//! that holds no batch does not; a batch edited through [`LaneSink::lane_batch_mut`] is marked
//! by the caller with [`LaneSink::mark_damage`].

use crate::layer_context::LayerContext;
use crate::packet_bindings::tex_bind_id;
use gpu_frame::frame::DrawBatch;
use render_primitives::frame::ids::{BindGroupId, LaneId};

/// The renderer's side of a lane.
///
/// `LaneTexture` is the record a textured lane keeps beside its batch: the texture whose lifetime
/// the batch's bind group depends on, and whatever the renderer reports about it. It is a type
/// parameter rather than an associated type so a renderer may keep a record type private to
/// itself.
pub trait LaneSink<LaneTexture> {
    /// Insert `batch` in lane order, replacing the lane's previous batch and dropping its texture
    /// record, and mark the frame damaged.
    fn upsert_lane_batch(&mut self, batch: DrawBatch);

    /// Remove `lane`'s batch and texture record; mark the frame damaged when a batch was removed.
    fn remove_lane_batch(&mut self, lane: LaneId);

    /// Upsert a textured lane's `batch` (as [`LaneSink::upsert_lane_batch`]) and keep `texture`
    /// beside it until the lane is next upserted or removed.
    fn upsert_textured_lane_batch(&mut self, batch: DrawBatch, texture: LaneTexture);

    /// The texture record of `lane`, if it is a live textured lane.
    fn lane_texture(&self, lane: LaneId) -> Option<&LaneTexture>;

    /// The batch `lane` holds, if any.
    fn lane_batch(&self, lane: LaneId) -> Option<&DrawBatch>;

    /// The batch `lane` holds, to change its visibility or buffers in place; the caller marks
    /// damage when the change shows.
    fn lane_batch_mut(&mut self, lane: LaneId) -> Option<&mut DrawBatch>;

    /// Mark the frame damaged so the next frame is drawn.
    fn mark_damage(&mut self);

    /// Lend the layer the renderer's device, queue, surface format, packet tables and statistics.
    fn layer_context(&mut self) -> LayerContext<'_>;

    /// The bind-group slot a textured `lane`'s batch names for its texture.
    fn textured_lane_binding(&self, lane: LaneId) -> BindGroupId {
        tex_bind_id(lane)
    }
}
