//! **Role:** the world layers' textured lane: `TexLane`, the texture record a textured lane keeps
//! beside its batch, and the upsert of a one-quad textured lane.
//! **Position:** the terrain texture, forest and terrain line of sight overlay layers build their
//! lanes through it, and the map renderer keeps the records through its
//! `renderer_core::lane_sink::LaneSink<TexLane>` implementation and reads them through the
//! accessors.
//! **Signals & state:** none; a record owns its texture and bind group.
//! **Invariants:** a record lives exactly as long as its lane's batch; its fields are written only
//! by the layers of this crate; the pipeline a lane draws with (`textured_quad`) travels to the
//! renderer as the batch's `PipelineId`.

use crate::basemap_mode::BasemapMode;
use crate::textured_quad::textured_pipeline_for;
use gpu_frame::frame::{DrawBatch, DrawPayload, InstanceBuffer};
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use renderer_core::lane_sink::LaneSink;

/// The record a textured lane keeps beside its batch: the texture handle to destroy, the bind
/// group the batch's texture slot binds, and the mode, tile and byte counters the statistics
/// report reads. The quad's instance buffer travels on the batch's
/// `DrawPayload::TexturedRect { instances, .. }`.
pub struct TexLane {
    /// The texture the lane samples.
    pub(crate) texture: wgpu::Texture,

    /// The bind group of the texture and its sampler.
    pub(crate) bind_group: wgpu::BindGroup,

    /// How the texture is laid out.
    pub(crate) mode: BasemapMode,

    /// The tiles written into the texture.
    pub(crate) tiles: u32,

    /// The texture's size in bytes, mip chain included.
    pub(crate) bytes: u64,
}

impl TexLane {
    /// The texture the lane samples, for its holder to destroy.
    #[must_use]
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// The bind group the batch's texture slot binds.
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// How the texture is laid out.
    #[must_use]
    pub fn mode(&self) -> BasemapMode {
        self.mode
    }

    /// The tiles written into the texture.
    #[must_use]
    pub fn tiles(&self) -> u32 {
        self.tiles
    }

    /// The texture's size in bytes, mip chain included.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}

/// Upsert `role`'s textured lane: one quad from `instances`, drawn with the pipeline `role`
/// draws with and the lane's own texture slot, plus the texture record the batch cannot carry,
/// through [`LaneSink::upsert_textured_lane_batch`].
pub(crate) fn upsert_textured_quad_lane(
    lanes: &mut dyn LaneSink<TexLane>,
    role: LaneRole,
    visible: bool,
    instances: wgpu::Buffer,
    texture: TexLane,
) {
    let lane = lane_id(role);
    let batch = DrawBatch {
        lane,
        visible,
        pipeline: textured_pipeline_for(role),
        payload: DrawPayload::TexturedRect {
            instances: InstanceBuffer::whole(instances, 32, 1),
            texture: lanes.textured_lane_binding(lane),
        },
    };
    lanes.upsert_textured_lane_batch(batch, texture);
}
