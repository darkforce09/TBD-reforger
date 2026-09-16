//! Role: gpu.
//! Position: `symbology/atlas` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;

use wasm_bindgen::prelude::*;

/// Glyph atlas gpu.
pub(crate) struct GlyphAtlasGpu {
    /// Texture.
    pub(crate) texture: wgpu::Texture,

    /// Uniform buf.
    pub(crate) uniform_buf: wgpu::Buffer,

    /// Bind group.
    pub(crate) bind_group: wgpu::BindGroup,

    /// Bytes.
    pub(crate) bytes: u64,
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload glyph atlas.
    pub fn upload_glyph_atlas(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        uv: &[f32],
    ) -> Result<(), JsError> {
        use crate::renderers::batching::scene::ATLAS_GLYPH_COUNT;

        if uv.len() > ATLAS_GLYPH_COUNT * 4 {
            return Err(JsError::new(&format!(
                "glyph-atlas-uv-count: capacity {}, got {}",
                ATLAS_GLYPH_COUNT * 4,
                uv.len()
            )));
        }
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .unwrap_or(0);
        if rgba.len() != expected {
            return Err(JsError::new("glyph-atlas-rgba-size"));
        }
        use wgpu::util::DeviceExt;
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph-atlas"),
            size: wgpu::Extent3d {
                width,
                height,
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
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let u_bytes = Self::pack_icon_uniforms(uv, 0.0, 0.0, 1.0);
        let uniform_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("glyph-icon-uniforms"),
                contents: &u_bytes,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph-atlas"),
            layout: &self.icon_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.icon_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        });
        if let Some(old) = self.glyph_atlas.take() {
            old.texture.destroy();
            old.uniform_buf.destroy();
        }
        self.glyph_atlas = Some(GlyphAtlasGpu {
            texture,
            uniform_buf,
            bind_group,
            bytes: expected as u64,
        });
        Ok(())
    }
}
