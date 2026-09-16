//! Role: overlay.
//! Position: `spatial/los/terrain` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::frame::engine::BasemapMode;
use crate::frame::engine::RenderEngine;
use crate::overlay::lanes::LaneRole;

use crate::world::terrain::satellite::textures::TexLane;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl RenderEngine {
    /// Viewshed upload.
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
    ) -> Result<(), JsError> {
        if tex_w == 0 || tex_h == 0 {
            return Err(JsError::new("viewshed_upload: zero texture dimensions"));
        }
        if bytes_per_row < tex_w * 4 || !bytes_per_row.is_multiple_of(256) {
            return Err(JsError::new(
                "viewshed_upload: bytes_per_row must be ≥ tex_w*4 and 256-aligned",
            ));
        }
        let want = (bytes_per_row as usize)
            .checked_mul(tex_h as usize)
            .ok_or_else(|| JsError::new("viewshed_upload: size overflow"))?;
        if rgba.len() != want {
            return Err(JsError::new("viewshed_upload: rgba length mismatch"));
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
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
        let rect = crate::world::scene::world_rect_rel([min_x, min_y], [max_x, max_y]);

        let inst = website_graphics_engine::layout::QuadInstance {
            min: [rect[0], rect[1]],
            max: [rect[2], rect[3]],
            color: [1.0, 1.0, 1.0, 1.0],
        };
        use wgpu::util::DeviceExt;
        let instances = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("viewshed-quad"),
                contents: bytemuck::cast_slice(&[inst]),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewshed"),
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
        self.upsert_textured_lane(LaneRole::Viewshed, true, instances, lane);
        Ok(())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Viewshed clear.
    pub fn viewshed_clear(&mut self) {
        self.remove_lane(LaneRole::Viewshed);
    }
}
