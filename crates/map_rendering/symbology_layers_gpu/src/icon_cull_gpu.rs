//! **Role:** [`IconCullGpu`], the compute frustum cull of the icon lanes: the GPU compaction,
//! whether the renderer may use it, and the CPU copy of the tree icons its oracle counts.
//! **Position:** `symbology_layers_gpu`; the renderer holds it as a field, builds it at
//! boot, encodes its cull pass before the main pass and draws its compacted lanes indirectly;
//! the slot and world icon lane uploads feed it through [`IconCullGpu::upload_lane`].
//! **Signals & state:** the `gpu_frame` compute cull (per-lane source and compacted buffers,
//! counters, the debug HUD flag), the cull-trees switch and the tree icons' packed bytes.
//! **Invariants:** the cull is in use only when the compute cull and its 32-bit storage icon
//! pipeline both exist and the cull-trees switch is on; a lane id is the lane role's `u32`
//! discriminant; with no compute cull every call is a no-op and every count reads zero.

use gpu_frame::draw::cull::compute::IconComputeCull;

/// The compute cull of the icon lanes and the state that decides whether it is in use.
pub struct IconCullGpu {
    /// The GPU compaction, absent on a WebGL2 backend.
    cull: Option<IconComputeCull>,

    /// Whether the renderer built the 32-bit storage icon pipeline that draws compacted lanes.
    storage_pipeline_ready: bool,

    /// Whether icon lanes route through the cull.
    cull_trees: bool,

    /// The anchor-relative 20-byte tree instances, kept for the CPU oracle count.
    tree_icons: Vec<u8>,
}

impl IconCullGpu {
    /// A cull over `cull` (absent on WebGL2), in use when `storage_pipeline_ready` and
    /// `cull_trees` hold.
    #[must_use]
    pub fn new(
        cull: Option<IconComputeCull>,
        storage_pipeline_ready: bool,
        cull_trees: bool,
    ) -> Self {
        Self {
            cull,
            storage_pipeline_ready,
            cull_trees,
            tree_icons: Vec::new(),
        }
    }

    /// Whether icon lanes route through the compute cull.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.cull_trees && self.cull.is_some() && self.storage_pipeline_ready
    }

    /// Whether any lane holds source instances to cull.
    #[must_use]
    pub fn has_any_source(&self) -> bool {
        self.cull.as_ref().is_some_and(IconComputeCull::has_any_src)
    }

    /// Upload `bytes` (anchor-relative 20-byte instances) as `lane`'s cull source.
    pub fn upload_lane(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        lane: u32,
        bytes: &[u8],
    ) {
        if let Some(cull) = &mut self.cull {
            cull.upload_lane(device, queue, lane, bytes);
        }
    }

    /// Empty `lane`'s cull source.
    pub fn clear_lane(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, lane: u32) {
        if let Some(cull) = &mut self.cull {
            cull.upload_lane(device, queue, lane, &[]);
        }
    }

    /// Encode the cull pass over `frustum` (anchor-relative `[min_x, min_y, max_x, max_y]`).
    pub fn encode_cull(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frustum: [f64; 4],
    ) {
        if let Some(cull) = &mut self.cull {
            cull.encode_cull(encoder, device, queue, frustum);
        }
    }

    /// Start the counter readback of the cull pass just submitted.
    pub fn kick_readback(&self) {
        if let Some(cull) = &self.cull {
            cull.kick_readback();
        }
    }

    /// `lane`'s compacted instance buffer and indirect-draw arguments, when the lane is culled.
    #[must_use]
    pub fn lane_draw(&self, lane: u32) -> Option<(&wgpu::Buffer, &wgpu::Buffer)> {
        self.cull.as_ref()?.lane_draw(lane)
    }

    /// Switch the cull's CPU oracle comparison (the debug HUD) on or off.
    pub fn set_debug_hud(&mut self, on: bool) {
        if let Some(cull) = &mut self.cull {
            cull.set_debug_hud(on);
        }
    }

    /// The CPU oracle count of the last cull frustum (zero while the debug HUD is off).
    #[must_use]
    pub fn last_cpu_count(&self) -> u32 {
        self.cull.as_ref().map(|c| c.last_cpu_count()).unwrap_or(0)
    }

    /// The GPU count of surviving instances across lanes, from the last landed readback.
    #[must_use]
    pub fn gpu_count(&self) -> u32 {
        self.cull
            .as_ref()
            .map(|c| c.gpu_count_for_stats())
            .unwrap_or(0)
    }

    /// The GPU count of `lane`'s surviving instances, from the last landed readback.
    #[must_use]
    pub fn lane_gpu_count(&self, lane: u32) -> u32 {
        self.cull
            .as_ref()
            .map(|c| c.lane_gpu_count_for_stats(lane))
            .unwrap_or(0)
    }

    /// Whether at least one real GPU counter readback has landed.
    #[must_use]
    pub fn gpu_sampled(&self) -> bool {
        self.cull.as_ref().is_some_and(|c| c.gpu_sampled())
    }

    /// Keep `bytes` as the tree icons the CPU oracle counts.
    pub fn set_tree_icons(&mut self, bytes: Vec<u8>) {
        self.tree_icons = bytes;
    }

    /// Drop the tree icons the CPU oracle counts.
    pub fn clear_tree_icons(&mut self) {
        self.tree_icons.clear();
    }

    /// The CPU oracle count of the tree icons inside `frustum` (anchor-relative).
    #[must_use]
    pub fn count_tree_icons_in_frustum(&self, frustum: [f64; 4]) -> u32 {
        render_primitives::draw::cull::oracle::count_icons_in_frustum(&self.tree_icons, frustum)
    }
}
