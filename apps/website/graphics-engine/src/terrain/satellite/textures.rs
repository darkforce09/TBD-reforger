//! Role: textures.
//! Position: `terrain/satellite` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::BasemapMode;
use crate::core::context::state::RenderEngine;
use crate::core::pipeline::draw_order::LaneRole;
use crate::renderers::batching::batch::Batch;
use crate::renderers::batching::batch::BatchPayload;

use crate::renderers::batching::scene::QuadInstance;
use wasm_bindgen::prelude::*;

/// Tex lane.
pub(crate) struct TexLane {
    /// Texture.
    pub(crate) texture: wgpu::Texture,

    /// Bind group.
    pub(crate) bind_group: wgpu::BindGroup,

    /// Instances.
    pub(crate) instances: wgpu::Buffer,

    /// Mode.
    pub(crate) mode: BasemapMode,

    /// Tiles.
    pub(crate) tiles: u32,

    /// Bytes.
    pub(crate) bytes: u64,
}

/// Pending tex.
pub(crate) struct PendingTex {
    /// Texture.
    pub(crate) texture: wgpu::Texture,

    /// World min.
    pub(crate) world_min: [f64; 2],

    /// World max.
    pub(crate) world_max: [f64; 2],

    /// Mode.
    pub(crate) mode: BasemapMode,

    /// Tiles.
    pub(crate) tiles: u32,

    /// Bytes.
    pub(crate) bytes: u64,
}

#[wasm_bindgen]
impl RenderEngine {
    /// Tex layer begin.
    #[allow(clippy::too_many_arguments)]
    pub fn tex_layer_begin(
        &mut self,
        role: u32,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        tex_w: u32,
        tex_h: u32,
        mip_count: u32,
        mode: u32,
    ) -> Result<(), JsError> {
        let idx = role as usize;
        if idx > 1 {
            return Err(JsError::new(
                "tex_layer: role must be 0 (basemap) or 1 (hillshade)",
            ));
        }
        if tex_w == 0 || tex_h == 0 || mip_count == 0 {
            return Err(JsError::new("tex_layer: zero texture dimensions"));
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("basemap-lane"),
            size: wgpu::Extent3d {
                width: tex_w,
                height: tex_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,

            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let base_bytes = u64::from(tex_w) * u64::from(tex_h) * 4;
        let bytes = if mip_count > 1 {
            base_bytes * 4 / 3
        } else {
            base_bytes
        };
        self.pending[idx] = Some(PendingTex {
            texture,
            world_min: [min_x, min_y],
            world_max: [max_x, max_y],
            mode: BasemapMode::from_u32(mode),
            tiles: 0,
            bytes,
        });
        Ok(())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Tex layer write bitmap.
    #[allow(clippy::too_many_arguments)]
    pub fn tex_layer_write_bitmap(
        &mut self,
        role: u32,
        mip: u32,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        bmp: web_sys::ImageBitmap,
    ) -> Result<(), JsError> {
        let idx = role as usize;
        let queue = &self.queue;
        let pending = self
            .pending
            .get_mut(idx)
            .and_then(|p| p.as_mut())
            .ok_or_else(|| JsError::new("tex_layer_write_bitmap: begin not called"))?;
        queue.copy_external_image_to_texture(
            &wgpu::CopyExternalImageSourceInfo {
                source: wgpu::ExternalImageSource::ImageBitmap(bmp),
                origin: wgpu::Origin2d::ZERO,
                flip_y: false,
            },
            wgpu::CopyExternalImageDestInfo {
                texture: &pending.texture,
                mip_level: mip,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
                color_space: wgpu::PredefinedColorSpace::Srgb,
                premultiplied_alpha: false,
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        pending.tiles += 1;
        Ok(())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Tex layer write rgba.
    #[allow(clippy::too_many_arguments)]
    pub fn tex_layer_write_rgba(
        &mut self,
        role: u32,
        mip: u32,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        rgba: &[u8],
    ) -> Result<(), JsError> {
        let idx = role as usize;
        if rgba.len() != (w as usize) * (h as usize) * 4 {
            return Err(JsError::new("tex_layer_write_rgba: byte length != w*h*4"));
        }
        let queue = &self.queue;
        let pending = self
            .pending
            .get_mut(idx)
            .and_then(|p| p.as_mut())
            .ok_or_else(|| JsError::new("tex_layer_write_rgba: begin not called"))?;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &pending.texture,
                mip_level: mip,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        pending.tiles += 1;
        Ok(())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Finalize the pending texture for `role` into a drawn lane: a 1-instance world-rect quad tinted `[1,1,1,opacity]` over the uploaded texture, upserted into the draw list in order.
    pub fn tex_layer_commit(
        &mut self,
        role: u32,
        opacity: f32,
        visible: bool,
    ) -> Result<(), JsError> {
        let idx = role as usize;
        let pending = self
            .pending
            .get_mut(idx)
            .and_then(Option::take)
            .ok_or_else(|| JsError::new("tex_layer_commit: begin not called"))?;
        let rect =
            crate::renderers::batching::lanes::world_rect_rel(pending.world_min, pending.world_max);
        let inst = QuadInstance {
            min: [rect[0], rect[1]],
            max: [rect[2], rect[3]],
            color: [1.0, 1.0, 1.0, opacity.clamp(0.0, 1.0)],
        };
        use wgpu::util::DeviceExt;
        let instances = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tex-lane-quad"),
                contents: bytemuck::cast_slice(&[inst]),

                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let view = pending
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tex-lane"),
            layout: &self.tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let lane = TexLane {
            texture: pending.texture,
            bind_group,
            instances,
            mode: pending.mode,
            tiles: pending.tiles,
            bytes: pending.bytes,
        };
        let role_enum = if idx == 0 {
            LaneRole::Satellite
        } else {
            LaneRole::Hillshade
        };
        self.upsert_lane(
            role_enum,
            Batch {
                role: role_enum,
                visible,
                payload: BatchPayload::Textured(lane),
            },
        );
        Ok(())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Drop the lane for `role` (and any half-uploaded pending texture) — e.g. a unified→pyramid re-resolve, or hillshade toggled off.
    pub fn tex_layer_clear(&mut self, role: u32) {
        let idx = role as usize;
        if idx > 1 {
            return;
        }
        self.pending[idx] = None;
        let role_enum = if idx == 0 {
            LaneRole::Satellite
        } else {
            LaneRole::Hillshade
        };
        self.remove_lane(role_enum);
    }
}
