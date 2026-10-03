//! **Role:** the pooled sprite lanes: write a lane's packed icons into its persistent pooled
//! buffer and upsert (or patch in place) the lane's sprite batch, or hand the icons to the compute
//! cull when it is in use.
//! **Position:** `symbology_layers_gpu::slot_symbology`, methods of [`SlotSymbology`]; every
//! slot, drag, cluster, preview, vehicle and comment upload ends here. [`is_pooled_icon_lane`]
//! tells a renderer which dropped batches draw from the pool.
//! **Signals & state:** the pooled lane buffers and the compute cull's lane sources.
//! **Invariants:** a pooled lane's buffer is owned by the pool and never destroyed with its
//! batch; a lane is 20-byte instances, and a byte count off the stride drops the lane; with the
//! compute cull in use the lane's batch is removed and the cull draws it.

use super::view::SlotSymbology;
use crate::icon_uniforms::{convert_icon_world_to_anchor, sprite_atlas_for};
use gpu_frame::frame::{DrawBatch, DrawPayload, InstanceBuffer};
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use render_primitives::frame::ids::LaneId;
use renderer_core::packet_bindings;

/// Whether `lane` draws out of the shared lane pool, and so must NOT have its buffer
/// destroyed when a batch is dropped. A dropped `DrawBatch` carries a `LaneId`, not a role, so
/// the question is asked in the batch's own terms.
#[must_use]
pub fn is_pooled_icon_lane(lane: LaneId) -> bool {
    [
        LaneRole::Slots,
        LaneRole::SlotDrag,
        LaneRole::Clusters,
        LaneRole::SlotPlacePreview,
        LaneRole::MissionVehicles,
        LaneRole::MissionComments,
    ]
    .into_iter()
    .any(|r| lane_id(r) == lane)
}

impl SlotSymbology<'_> {
    /// Upsert pooled icon lane.
    pub(super) fn upsert_pooled_icon_lane(
        &mut self,
        role: LaneRole,
        buf: wgpu::Buffer,
        count: u32,
        visible: bool,
        buffer_changed: bool,
    ) {
        let lane = lane_id(role);
        if !buffer_changed
            && let Some(batch) = self.lanes.lane_batch_mut(lane)
            && let DrawPayload::Sprites { instances, .. } = &mut batch.payload
        {
            instances.buffer = buf;
            instances.count = count;
            batch.visible = visible;
            self.lanes.mark_damage();
            return;
        }
        self.upsert_lane(
            role,
            DrawBatch {
                lane,
                visible,
                pipeline: packet_bindings::PIPE_ICON,
                payload: DrawPayload::Sprites {
                    instances: InstanceBuffer::whole(buf, 20, count),
                    atlas: sprite_atlas_for(role),
                },
            },
        );
    }

    /// Upload slot role lane.
    pub(super) fn upload_slot_role_lane(&mut self, role: LaneRole, bytes: &[u8], visible: bool) {
        const STRIDE: usize = 20;
        if bytes.is_empty() {
            if !visible {
                self.clear_cull_lane(role);
                self.remove_lane(role);
            }
            return;
        }
        if !bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(role);
            return;
        }
        #[allow(clippy::cast_possible_truncation)]
        let count = (bytes.len() / STRIDE) as u32;

        let context = self.lanes.layer_context();
        let (buf, buffer_changed) = self.lane_pool.write_gpu(
            context.device(),
            context.queue(),
            role as u32,
            bytes,
            convert_icon_world_to_anchor,
        );
        if self.icon_cull.enabled() {
            let packed = self.lane_pool.contents(role as u32).to_vec();
            self.icon_cull
                .upload_lane(context.device(), context.queue(), role as u32, &packed);
            let _ = (buf, buffer_changed, count);
            self.remove_lane(role);
            self.lanes.mark_damage();
            return;
        }
        self.upsert_pooled_icon_lane(role, buf, count, visible, buffer_changed);
    }
}
