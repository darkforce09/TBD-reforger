//! Role: buffers.
//! Position: `world/environment/vegetation` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::BasemapMode;
use crate::core::context::state::RenderEngine;
use crate::overlay::lanes::LaneRole;
use crate::overlay::lanes::lane_id;

use crate::world::terrain::satellite::textures::TexLane;
use wasm_bindgen::prelude::*;
use website_graphics_engine::frame::DrawPayload;

#[wasm_bindgen]
impl RenderEngine {
    /// Forest density upload.
    #[allow(clippy::too_many_arguments)]
    pub fn forest_density_upload(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        tex_w: u32,
        tex_h: u32,
        rgba: &[u8],
        bytes_per_row: u32,
        bins_ok: u32,
    ) -> Result<(), JsError> {
        if tex_w == 0 || tex_h == 0 {
            return Err(JsError::new(
                "forest_density_upload: zero texture dimensions",
            ));
        }
        if bytes_per_row < tex_w * 4 || !bytes_per_row.is_multiple_of(256) {
            return Err(JsError::new(
                "forest_density_upload: bytes_per_row must be ≥ tex_w*4 and 256-aligned",
            ));
        }
        let want = (bytes_per_row as usize)
            .checked_mul(tex_h as usize)
            .ok_or_else(|| JsError::new("forest_density_upload: size overflow"))?;
        if rgba.len() != want {
            return Err(JsError::new("forest_density_upload: rgba length mismatch"));
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
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
        self.queue.write_texture(
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
        let rect =
            crate::renderers::batching::lanes::world_rect_rel([min_x, min_y], [max_x, max_y]);

        let inst = crate::renderers::batching::scene::QuadInstance {
            min: [rect[0], rect[1]],
            max: [rect[2], rect[3]],
            color: [0.0, 0.0, 0.0, 0.35],
        };
        use wgpu::util::DeviceExt;
        let instances = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("forest-density-quad"),
                contents: bytemuck::cast_slice(&[inst]),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        self.upsert_textured_lane(LaneRole::ForestFill, true, instances, lane);
        self.forest_density_w = tex_w;
        self.forest_density_h = tex_h;
        self.forest_bins_ok = bins_ok;
        self.forest_mode = "density".into();
        self.forest_polygons = 625;
        self.forest_outline_segments = 0;
        Ok(())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Forest density set params.
    pub fn forest_density_set_params(
        &mut self,
        fill_alpha: f32,
        fill_visible: bool,
        outline_visible: bool,
    ) {
        if self.forest_mode != "density" {
            return;
        }
        let color = [0.0, 0.0, 0.0, fill_alpha.clamp(0.0, 1.0)];
        let fill_lane = lane_id(LaneRole::ForestFill);
        let target = self.batches.iter_mut().find_map(|b| {
            if b.lane == fill_lane {
                b.visible = fill_visible;
                if let DrawPayload::TexturedRect { instances, .. } = &b.payload {
                    return Some(instances.buffer.clone());
                }
            }
            None
        });
        if let Some(buf) = target {
            self.queue
                .write_buffer(&buf, 16, bytemuck::cast_slice(&color));
            self.damage.mark();
        }
        let outline_lane = lane_id(LaneRole::ForestOutline);
        for b in &mut self.batches {
            if b.lane == outline_lane {
                b.visible = outline_visible && self.forest_outline_segments_stored > 0;
            }
        }
        self.forest_polygons = if fill_visible { 625 } else { 0 };
        self.forest_outline_segments = if outline_visible {
            self.forest_outline_segments_stored
        } else {
            0
        };
        self.damage.mark();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Forest outline set stored.
    pub fn forest_outline_set_stored(&mut self, segments: u32) {
        self.forest_outline_segments_stored = segments;
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Forest density clear.
    pub fn forest_density_clear(&mut self) {
        self.remove_lane(LaneRole::ForestFill);
        self.remove_lane(LaneRole::ForestOutline);
        self.forest_density_w = 0;
        self.forest_density_h = 0;
        self.forest_bins_ok = 0;
        self.forest_outline_segments_stored = 0;
        self.forest_mode.clear();
        self.forest_polygons = 0;
        self.forest_outline_segments = 0;
    }
}
