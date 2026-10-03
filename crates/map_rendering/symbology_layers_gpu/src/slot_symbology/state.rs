//! **Role:** [`SlotSymbologyGpu`], the slot symbology's owned GPU state: the movable-sprite
//! (slot) atlas, the slot bridge's cached columns, selection and drag, the shared pool the icon
//! lanes draw from, and the icon bind-group layout and sampler the atlas binds with.
//! **Position:** `symbology_layers_gpu::slot_symbology`; the renderer holds one as a field,
//! reads its atlas bind groups when it fills the frame packet, and lends it out as a
//! [`SlotSymbology`] (with the lane sink and the camera) for every bind, drag, selection and
//! camera change.
//! **Signals & state:** the slot atlas texture and its two uniform buffers, the bridge columns,
//! the pooled lane buffers.
//! **Invariants:** the atlas bind groups exist exactly while the atlas does; the bridge is armed
//! (`atlas_ready`) only after an atlas upload, and every bind before that only records its
//! columns.

use super::view::{SlotSymbology, TextAtlasSupply};
use crate::icon_cull_gpu::IconCullGpu;
use camera_math::ortho::state::OrthoCamera;
use gpu_device::buffers::pool::LanePool;
use renderer_core::lane_sink::LaneSink;
use std::collections::HashSet;
use std::convert::Infallible;

/// The slot bridge's cached columns, selection, drag and cluster state.
#[derive(Default)]
pub(crate) struct SlotGpuBridge {
    /// Whether the slot atlas is uploaded, so the slot lanes can be drawn.
    pub(crate) atlas_ready: bool,

    /// Slot ids, one per row of the slot lane.
    pub(crate) last_ids: Vec<String>,

    /// Slot world positions, two `f32` per row.
    pub(crate) last_xy: Vec<f32>,

    /// Slot side tints, one RGBA per row.
    pub(crate) last_side_tints: Vec<[u8; 4]>,

    /// The selected ids, slots and comments alike.
    pub(crate) selected_ids: HashSet<String>,

    /// Whether the slot lane last packed as clusters.
    pub(crate) last_cluster_mode: bool,

    /// Whether a drag overlay is live.
    pub(crate) drag_active: bool,

    /// The dragged ids.
    pub(crate) drag_ids: Vec<String>,

    /// Whether the slot lane holds only the selected rows (cluster mode).
    pub(crate) slots_lane_selection_only: bool,

    /// The cluster index over `last_xy`, rebuilt when the slot count changes.
    pub(crate) cluster_index: Option<spatial_indexes::point_indexes::cluster::ClusterIndex>,

    /// The slot count the cluster index was built over.
    pub(crate) cluster_built_len: usize,

    /// The first symbology cell of the widened atlas, when the atlas carries symbology cells.
    pub(crate) symbology_base: Option<u16>,

    /// Slot roles, one per row.
    pub(crate) last_roles: Vec<String>,

    /// Slot headings in degrees, one per row.
    pub(crate) last_headings: Vec<f32>,

    /// Comment world positions, two `f32` per comment.
    pub(crate) comment_xy: Vec<f32>,

    /// Comment ids, in the id space of the selection.
    pub(crate) comment_ids: Vec<String>,

    /// Whether the slot lane last packed detailed symbology rather than discs.
    pub(crate) last_symbology_detailed: bool,
}

/// The slot atlas on the GPU: its texture, the base and dragged uniform blocks and bind groups.
pub(crate) struct SlotAtlasGpu {
    /// The atlas texture.
    pub(crate) texture: wgpu::Texture,

    /// The uniform block the base lanes bind.
    pub(crate) base_uniform_buf: wgpu::Buffer,

    /// The uniform block the drag lane binds, which carries the drag offset.
    pub(crate) drag_uniform_buf: wgpu::Buffer,

    /// The base lanes' bind group.
    pub(crate) base_bind_group: wgpu::BindGroup,

    /// The drag lane's bind group.
    pub(crate) drag_bind_group: wgpu::BindGroup,

    /// The texture's byte size.
    pub(crate) bytes: u64,

    /// The pixels-to-metres scale both uniform blocks hold.
    pub(crate) px_to_m: f32,

    /// The drag offset the drag uniform block holds.
    pub(crate) drag_delta: [f32; 2],
}

/// The slot symbology's owned GPU state.
pub struct SlotSymbologyGpu {
    /// The slot atlas, once uploaded.
    pub(super) slot_atlas: Option<SlotAtlasGpu>,

    /// The slot bridge.
    pub(super) slot_bridge: SlotGpuBridge,

    /// The pooled buffers the slot, drag, cluster, preview, vehicle and comment lanes draw from.
    pub(super) lane_pool: LanePool,

    /// The icon bind-group layout the atlas bind groups are built against.
    pub(super) icon_bind_group_layout: wgpu::BindGroupLayout,

    /// The sampler the atlas bind groups bind.
    pub(super) icon_sampler: wgpu::Sampler,
}

impl SlotSymbologyGpu {
    /// No atlas, an empty bridge and pool; atlas bind groups are built against
    /// `icon_bind_group_layout` with `icon_sampler`.
    #[must_use]
    pub fn new(icon_bind_group_layout: wgpu::BindGroupLayout, icon_sampler: wgpu::Sampler) -> Self {
        Self {
            slot_atlas: None,
            slot_bridge: SlotGpuBridge::default(),
            lane_pool: LanePool::new(),
            icon_bind_group_layout,
            icon_sampler,
        }
    }

    /// Lend the state out with what a bind needs from the renderer: the icon cull, the lane sink,
    /// the camera, the shared text atlas and the uniform byte counter.
    pub fn at_work<'a>(
        &'a mut self,
        icon_cull: &'a mut IconCullGpu,
        lanes: &'a mut dyn LaneSink<Infallible>,
        camera: &'a OrthoCamera,
        text_atlas: &'a mut dyn TextAtlasSupply,
        uniform_bytes_last_frame: &'a mut u32,
    ) -> SlotSymbology<'a> {
        SlotSymbology {
            slot_bridge: &mut self.slot_bridge,
            slot_atlas: &mut self.slot_atlas,
            lane_pool: &mut self.lane_pool,
            icon_bind_group_layout: &self.icon_bind_group_layout,
            icon_sampler: &self.icon_sampler,
            icon_cull,
            lanes,
            camera,
            text_atlas,
            uniform_bytes_last_frame,
        }
    }

    /// Whether the slot atlas is uploaded and the slot lanes are armed.
    #[must_use]
    pub fn atlas_ready(&self) -> bool {
        self.slot_bridge.atlas_ready
    }

    /// Whether the slot atlas exists on the GPU.
    #[must_use]
    pub fn has_atlas(&self) -> bool {
        self.slot_atlas.is_some()
    }

    /// The slot atlas texture's byte size, zero without an atlas.
    #[must_use]
    pub fn atlas_bytes(&self) -> u64 {
        self.slot_atlas.as_ref().map_or(0, |a| a.bytes)
    }

    /// The bind group the base sprite lanes sample, once the atlas exists.
    #[must_use]
    pub fn atlas_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.slot_atlas.as_ref().map(|a| &a.base_bind_group)
    }

    /// The bind group the slot drag lane samples, once the atlas exists.
    #[must_use]
    pub fn dragged_atlas_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.slot_atlas.as_ref().map(|a| &a.drag_bind_group)
    }

    /// Forget every pooled lane buffer (the lanes drawing from them are dropped by the caller).
    pub fn clear_lane_pool(&mut self) {
        self.lane_pool.clear();
    }

    /// `engine_stats` (a JSON object) with the slot bridge's flags appended: the slot count, the
    /// cluster mode, the selection-only lane, the live drag and the armed atlas.
    #[must_use]
    pub fn append_slot_stats(&self, engine_stats: &str) -> String {
        let trimmed = engine_stats.trim_end_matches('}');
        format!(
            "{trimmed},\"slot_len\":{},\"cluster_mode\":{},\"slots_lane_selection_only\":{},\"drag_active\":{},\"atlas_ready\":{}}}",
            self.slot_bridge.last_ids.len(),
            if self.slot_bridge.last_cluster_mode {
                "true"
            } else {
                "false"
            },
            if self.slot_bridge.slots_lane_selection_only {
                "true"
            } else {
                "false"
            },
            if self.slot_bridge.drag_active {
                "true"
            } else {
                "false"
            },
            if self.slot_bridge.atlas_ready {
                "true"
            } else {
                "false"
            },
        )
    }
}
