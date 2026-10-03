//! **Role:** `TerrainLineOfSightOverlayGpu`, the terrain line of sight overlay typed layer: a
//! viewshed raster uploaded as the textured viewshed lane, and the lane's removal;
//! `TerrainLineOfSightOverlay`, the layer at work, lent the renderer's lanes.
//! **Position:** the map renderer holds one as a field and lends it with its lanes through
//! `RenderEngine::with_terrain_line_of_sight_overlay`, the door the Mission Creator's line of
//! sight tools and the building viewer upload and clear the wash through.
//! **Signals & state:** the textured bind-group layout and the linear sampler the raster binds
//! with; the uploaded raster itself is the viewshed lane's texture record, which the renderer
//! keeps.
//! **Invariants:** a viewshed raster is `bytes_per_row × height` bytes with a 256-aligned row of
//! at least four bytes per texel; an upload replaces the viewshed lane whole and always shows it;
//! clearing an absent lane changes nothing.

use crate::basemap_mode::BasemapMode;
use crate::error::{LayerCall, Result};
use crate::raster_layout::check_raster;
use crate::textured_lane::{TexLane, upsert_textured_quad_lane};
use crate::textured_quad::world_rect_rel;
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use renderer_core::lane_sink::LaneSink;

/// The viewshed lane's bind-group layout and sampler.
pub struct TerrainLineOfSightOverlayGpu {
    /// The textured bind-group layout the raster's bind group is built against.
    tex_bind_group_layout: wgpu::BindGroupLayout,

    /// The linear sampler the raster binds.
    density_sampler: wgpu::Sampler,
}

impl TerrainLineOfSightOverlayGpu {
    /// The raster's bind group is built against `tex_bind_group_layout` with `density_sampler`.
    #[must_use]
    pub fn new(
        tex_bind_group_layout: wgpu::BindGroupLayout,
        density_sampler: wgpu::Sampler,
    ) -> Self {
        Self {
            tex_bind_group_layout,
            density_sampler,
        }
    }

    /// The layer lent the renderer's `lanes` for one call.
    pub fn at_work<'a>(
        &'a self,
        lanes: &'a mut dyn LaneSink<TexLane>,
    ) -> TerrainLineOfSightOverlay<'a> {
        TerrainLineOfSightOverlay { layer: self, lanes }
    }
}

/// The terrain line of sight overlay lent the renderer's lanes.
pub struct TerrainLineOfSightOverlay<'a> {
    /// The layer's bind-group layout and sampler.
    layer: &'a TerrainLineOfSightOverlayGpu,

    /// The renderer's lanes.
    lanes: &'a mut dyn LaneSink<TexLane>,
}

impl TerrainLineOfSightOverlay<'_> {
    /// Upload the `tex_w`×`tex_h` viewshed raster `rgba` over the world rectangle
    /// `[min_x, min_y]`…`[max_x, max_y]` as the visible viewshed lane.
    ///
    /// # Errors
    /// [`crate::Error`] tagged `viewshed_upload`: a zero dimension, a row that is shorter than
    /// `tex_w × 4` bytes or not 256-aligned, a size overflow, or `rgba` not
    /// `bytes_per_row × tex_h` bytes long.
    #[allow(clippy::too_many_arguments)]
    pub fn viewshed_upload(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        tex_w: u32,
        tex_h: u32,
        rgba: &[u8],
        bytes_per_row: u32,
    ) -> Result<()> {
        check_raster(
            LayerCall::ViewshedUpload,
            tex_w,
            tex_h,
            bytes_per_row,
            rgba.len(),
        )?;
        let context = self.lanes.layer_context();
        let device = context.device();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("viewshed"),
            size: wgpu::Extent3d {
                width: tex_w,
                height: tex_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        context.queue().write_texture(
            texture.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(tex_h),
            },
            wgpu::Extent3d {
                width: tex_w,
                height: tex_h,
                depth_or_array_layers: 1,
            },
        );
        let rect = world_rect_rel([min_x, min_y], [max_x, max_y]);

        let inst = render_primitives::draw::instances::QuadInstance {
            min: [rect[0], rect[1]],
            max: [rect[2], rect[3]],
            color: [1.0, 1.0, 1.0, 1.0],
        };
        use wgpu::util::DeviceExt;
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewshed-quad"),
            contents: bytemuck::cast_slice(&[inst]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewshed"),
            layout: &self.layer.tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,

                    resource: wgpu::BindingResource::Sampler(&self.layer.density_sampler),
                },
            ],
        });
        let lane = TexLane {
            texture,
            bind_group,
            mode: BasemapMode::Single,
            tiles: 1,
            bytes: u64::from(bytes_per_row) * u64::from(tex_h),
        };
        upsert_textured_quad_lane(self.lanes, LaneRole::Viewshed, true, instances, lane);
        Ok(())
    }

    /// Remove the viewshed lane.
    pub fn viewshed_clear(&mut self) {
        self.lanes.remove_lane_batch(lane_id(LaneRole::Viewshed));
    }
}
