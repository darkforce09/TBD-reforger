//! **Role:** [`SlotSymbology`], the slot symbology at work: the owned state of a
//! [`super::SlotSymbologyGpu`] borrowed together with the renderer parts a bind writes through
//! (the lane sink, the icon cull, the camera, the shared text atlas, the uniform byte counter),
//! and the helpers every bind shares; [`TextAtlasSupply`], the renderer's shared text atlas.
//! **Position:** `symbology_layers_gpu::slot_symbology`; built by
//! [`super::SlotSymbologyGpu::at_work`] for the duration of one call, whose methods live in the
//! sibling files by concern (atlas, slot lane, drag, clusters, mission lanes, pooled lanes).
//! **Signals & state:** none of its own; every field is a borrow.
//! **Invariants:** lanes are written only through the lane sink, keyed by the lane role's
//! `LaneId`; the zoom is the camera's, read on every call, never cached here.

use super::state::{SlotAtlasGpu, SlotGpuBridge};
use crate::icon_cull_gpu::IconCullGpu;
use camera_math::ortho::state::OrthoCamera;
use gpu_device::buffers::pool::LanePool;
use gpu_frame::frame::DrawBatch;
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use renderer_core::lane_sink::LaneSink;
use std::convert::Infallible;

/// The renderer's shared text cell atlas, which a marker caption run samples.
pub trait TextAtlasSupply {
    /// Build the text atlas if the renderer holds none yet; whether one is in place after.
    fn ensure_text_atlas(&mut self) -> bool;
}

/// The slot symbology's state borrowed with the renderer parts its binds write through. Its lanes
/// are sprite lanes and carry no texture record, so the lane sink's texture type is `Infallible`.
pub struct SlotSymbology<'a> {
    /// The slot bridge.
    pub(super) slot_bridge: &'a mut SlotGpuBridge,

    /// The slot atlas, once uploaded.
    pub(super) slot_atlas: &'a mut Option<SlotAtlasGpu>,

    /// The pooled lane buffers.
    pub(super) lane_pool: &'a mut LanePool,

    /// The icon bind-group layout the atlas bind groups are built against.
    pub(super) icon_bind_group_layout: &'a wgpu::BindGroupLayout,

    /// The sampler the atlas bind groups bind.
    pub(super) icon_sampler: &'a wgpu::Sampler,

    /// The compute cull the pooled lanes feed when it is in use.
    pub(super) icon_cull: &'a mut IconCullGpu,

    /// The renderer's lanes.
    pub(super) lanes: &'a mut dyn LaneSink<Infallible>,

    /// The renderer's camera, whose zoom gates the symbology and clusters.
    pub(super) camera: &'a OrthoCamera,

    /// The renderer's shared text atlas.
    pub(super) text_atlas: &'a mut dyn TextAtlasSupply,

    /// The renderer's count of uniform bytes written for the last frame.
    pub(super) uniform_bytes_last_frame: &'a mut u32,
}

impl SlotSymbology<'_> {
    /// The camera's zoom.
    pub(super) fn zoom(&self) -> f64 {
        self.camera.zoom()
    }

    /// Metres per screen pixel at the camera's zoom.
    pub(super) fn slot_m_per_px(&self) -> f32 {
        map_draw_lanes::zoom_gates::px_to_m_at_zoom(self.zoom())
    }

    /// Whether the camera is close enough for detailed symbology rather than discs.
    pub(super) fn symbology_detailed(&self) -> bool {
        overlay_instances::symbols::symbology_visible(self.slot_m_per_px())
    }

    /// Upsert `role`'s lane with `batch`, whose lane is `role`'s lane id.
    pub(super) fn upsert_lane(&mut self, role: LaneRole, batch: DrawBatch) {
        debug_assert_eq!(
            batch.lane,
            lane_id(role),
            "a batch is keyed by its role's lane"
        );
        self.lanes.upsert_lane_batch(batch);
    }

    /// Remove `role`'s lane.
    pub(super) fn remove_lane(&mut self, role: LaneRole) {
        self.lanes.remove_lane_batch(lane_id(role));
    }

    /// Empty `role`'s compute cull source.
    pub(super) fn clear_cull_lane(&mut self, role: LaneRole) {
        let context = self.lanes.layer_context();
        self.icon_cull
            .clear_lane(context.device(), context.queue(), role as u32);
    }
}
