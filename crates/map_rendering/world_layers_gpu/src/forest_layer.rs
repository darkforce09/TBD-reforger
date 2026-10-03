//! **Role:** `ForestLayerGpu`, the forest typed layer: the forest density raster uploaded as
//! the textured forest fill lane, the fill and outline settings applied to the forest lanes, and
//! the forest counters the statistics report reads.
//! **Position:** the map renderer holds one as a field, its asset sink forwards the forest mass
//! loader's density upload and settings here with the renderer's lanes lent as a
//! `renderer_core::lane_sink::LaneSink`, and the renderer's vector lane uploads record the vector
//! forest counts through `ForestLayerGpu::record_fill_polygons` and
//! `ForestLayerGpu::record_outline_segments`.
//! **Signals & state:** the density raster's size and loaded bins, the forest mode, the drawn and
//! stored outline segment counts and the drawn fill polygon count; the textured bind-group layout
//! and the density sampler the raster binds with.
//! **Invariants:** a density raster is `bytes_per_row × height` bytes with a 256-aligned row of at
//! least four bytes per texel; the settings apply only in density mode; the outline lane shows only
//! when outline segments are stored.

use crate::basemap_mode::BasemapMode;
use crate::error::{LayerCall, Result};
use crate::raster_layout::check_raster;
use crate::textured_lane::{TexLane, upsert_textured_quad_lane};
use crate::textured_quad::world_rect_rel;
use gpu_frame::frame::DrawPayload;
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use renderer_core::lane_sink::LaneSink;

/// The forest density lane, the forest lane settings and the forest counters.
pub struct ForestLayerGpu {
    /// The textured bind-group layout the raster's bind group is built against.
    tex_bind_group_layout: wgpu::BindGroupLayout,

    /// The linear sampler the raster binds.
    density_sampler: wgpu::Sampler,

    /// The forest fill polygons drawn.
    polygons: u32,

    /// The forest outline segments drawn.
    outline_segments: u32,

    /// The density raster's width in texels.
    density_width: u32,

    /// The density raster's height in texels.
    density_height: u32,

    /// The density bins the loader loaded.
    bins_loaded: u32,

    /// The outline segments the loader stored, shown when the outline is visible.
    outline_segments_stored: u32,

    /// `"density"` once a density raster is uploaded, else empty.
    mode: &'static str,
}

impl ForestLayerGpu {
    /// No raster yet; the raster's bind group is built against `tex_bind_group_layout` with
    /// `density_sampler`.
    #[must_use]
    pub fn new(
        tex_bind_group_layout: wgpu::BindGroupLayout,
        density_sampler: wgpu::Sampler,
    ) -> Self {
        Self {
            tex_bind_group_layout,
            density_sampler,
            polygons: 0,
            outline_segments: 0,
            density_width: 0,
            density_height: 0,
            bins_loaded: 0,
            outline_segments_stored: 0,
            mode: "",
        }
    }

    /// The forest fill polygons drawn.
    #[must_use]
    pub fn polygons(&self) -> u32 {
        self.polygons
    }

    /// The forest outline segments drawn.
    #[must_use]
    pub fn outline_segments(&self) -> u32 {
        self.outline_segments
    }

    /// The density raster's width in texels.
    #[must_use]
    pub fn density_width(&self) -> u32 {
        self.density_width
    }

    /// The density raster's height in texels.
    #[must_use]
    pub fn density_height(&self) -> u32 {
        self.density_height
    }

    /// The density bins the loader loaded.
    #[must_use]
    pub fn bins_loaded(&self) -> u32 {
        self.bins_loaded
    }

    /// `"density"` once a density raster is uploaded, else empty.
    #[must_use]
    pub fn mode(&self) -> &'static str {
        self.mode
    }

    /// Record the polygons a vector forest fill lane draws.
    pub fn record_fill_polygons(&mut self, polygons: u32) {
        self.polygons = polygons;
    }

    /// Record the segments a vector forest outline lane draws.
    pub fn record_outline_segments(&mut self, segments: u32) {
        self.outline_segments = segments;
    }

    /// Upload the `tex_w`×`tex_h` density raster `rgba` over the world rectangle
    /// `[min_x, min_y]`…`[max_x, max_y]` as the visible forest fill lane, and switch to density
    /// mode.
    ///
    /// # Errors
    /// [`crate::Error`] tagged `forest_density_upload`: a zero dimension, a row that is shorter
    /// than `tex_w × 4` bytes or not 256-aligned, a size overflow, or `rgba` not
    /// `bytes_per_row × tex_h` bytes long.
    #[allow(clippy::too_many_arguments)]
    pub fn upload_density(
        &mut self,
        lanes: &mut dyn LaneSink<TexLane>,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        tex_w: u32,
        tex_h: u32,
        rgba: &[u8],
        bytes_per_row: u32,
        bins_ok: u32,
    ) -> Result<()> {
        check_raster(
            LayerCall::ForestDensityUpload,
            tex_w,
            tex_h,
            bytes_per_row,
            rgba.len(),
        )?;
        let context = lanes.layer_context();
        let device = context.device();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("forest-density"),
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
            color: [0.0, 0.0, 0.0, 0.35],
        };
        use wgpu::util::DeviceExt;
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("forest-density-quad"),
            contents: bytemuck::cast_slice(&[inst]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forest-density"),
            layout: &self.tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.density_sampler),
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
        upsert_textured_quad_lane(lanes, LaneRole::ForestFill, true, instances, lane);
        self.density_width = tex_w;
        self.density_height = tex_h;
        self.bins_loaded = bins_ok;
        self.mode = "density";
        self.polygons = 625;
        self.outline_segments = 0;
        Ok(())
    }

    /// Apply the fill alpha and the fill and outline visibility to the forest lanes, in density
    /// mode only.
    pub fn set_params(
        &mut self,
        lanes: &mut dyn LaneSink<TexLane>,
        fill_alpha: f32,
        fill_visible: bool,
        outline_visible: bool,
    ) {
        if self.mode != "density" {
            return;
        }
        let color = [0.0, 0.0, 0.0, fill_alpha.clamp(0.0, 1.0)];
        let target = lanes
            .lane_batch_mut(lane_id(LaneRole::ForestFill))
            .and_then(|b| {
                b.visible = fill_visible;
                if let DrawPayload::TexturedRect { instances, .. } = &b.payload {
                    return Some(instances.buffer.clone());
                }
                None
            });
        if let Some(buf) = target {
            lanes
                .layer_context()
                .queue()
                .write_buffer(&buf, 16, bytemuck::cast_slice(&color));
            lanes.mark_damage();
        }
        if let Some(b) = lanes.lane_batch_mut(lane_id(LaneRole::ForestOutline)) {
            b.visible = outline_visible && self.outline_segments_stored > 0;
        }
        self.polygons = if fill_visible { 625 } else { 0 };
        self.outline_segments = if outline_visible {
            self.outline_segments_stored
        } else {
            0
        };
        lanes.mark_damage();
    }

    /// Store the outline segment count the loader uploaded, shown when the outline is visible.
    pub fn set_outline_stored(&mut self, segments: u32) {
        self.outline_segments_stored = segments;
    }
}
