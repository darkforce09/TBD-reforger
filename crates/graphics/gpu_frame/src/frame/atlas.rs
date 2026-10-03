//! **Role:** the cell-atlas GPU handles (texture, uniform buffer, bind group) of the text atlas
//! and the glyph atlas, and the constructors that build them.
//! **Position:** `frame` in the GPU frame crate: frame vocabulary, because a cell atlas's
//! `bind_group` is what a [`crate::frame::text::TextRun`]'s `atlas: BindGroupId` resolves to.
//! The map engine's render engine owns the atlas slots.
//! **Signals & state:** the two cell-atlas textures, their uniform buffers and their bind groups.
//! **Invariants:** an atlas here is a grid of cells with a uniform block. What a cell depicts —
//! a letter, a unit, a vehicle — is the caller's business and never reaches this module.
//!
//! The handles are live GPU resources, so they sit here rather than in `text`, which holds glyph
//! shapes and byte layouts only. The uniform-block packing, `TEXT_UNIFORM_BYTES` and
//! `text_uniform_bytes()`, names no `wgpu` type and lives with the rest of the byte layout in
//! `render_primitives::text::pack`.

use crate::error::{Error, Result};

/// Text atlas gpu.
pub struct TextAtlasGpu {
    /// Texture.
    pub texture: wgpu::Texture,

    /// Uniform buf.
    pub uniform_buf: wgpu::Buffer,

    /// Bind group.
    pub bind_group: wgpu::BindGroup,

    /// Bytes.
    pub bytes: u64,
}

/// Glyph atlas gpu.
pub struct GlyphAtlasGpu {
    /// Texture.
    pub texture: wgpu::Texture,

    /// Uniform buf.
    pub uniform_buf: wgpu::Buffer,

    /// Bind group.
    pub bind_group: wgpu::BindGroup,

    /// Bytes.
    pub bytes: u64,
}

fn upload_cell_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &'static str,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<(wgpu::Texture, u64)> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .unwrap_or(0);
    if rgba.len() != expected {
        return Err(Error::AtlasPixelLength {
            atlas: label,
            width,
            height,
            actual: rgba.len(),
        });
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
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
    queue.write_texture(
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
    Ok((texture, expected as u64))
}

fn cell_bind_group(
    device: &wgpu::Device,
    label: &str,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform_buf: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform_buf.as_entire_binding(),
            },
        ],
    })
}

/// Build the text cell atlas: texture upload, `TextUniforms` buffer, bind group.
///
/// # Errors
/// [`Error::AtlasPixelLength`] (message `text-atlas-rgba-size`) when `rgba` is not
/// `width × height × 4` bytes long.
pub fn create_text_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<TextAtlasGpu> {
    use wgpu::util::DeviceExt;
    let (texture, bytes) = upload_cell_texture(device, queue, "text-atlas", rgba, width, height)?;
    let u_bytes = render_primitives::text::pack::text_uniform_bytes();
    let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("text-uniforms"),
        contents: &u_bytes,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = cell_bind_group(device, "text-atlas", layout, &view, sampler, &uniform_buf);
    Ok(TextAtlasGpu {
        texture,
        uniform_buf,
        bind_group,
        bytes,
    })
}

/// Build the glyph cell atlas from an already-packed uniform block.
///
/// `uniform_bytes` arrives packed because its UV table is the caller's cell layout; this
/// function never reads it.
///
/// # Errors
/// [`Error::AtlasPixelLength`] (message `glyph-atlas-rgba-size`) when `rgba` is not
/// `width × height × 4` bytes long.
#[allow(clippy::too_many_arguments)]
pub fn create_glyph_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    rgba: &[u8],
    width: u32,
    height: u32,
    uniform_bytes: &[u8],
) -> Result<GlyphAtlasGpu> {
    use wgpu::util::DeviceExt;
    let (texture, bytes) = upload_cell_texture(device, queue, "glyph-atlas", rgba, width, height)?;
    let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("glyph-icon-uniforms"),
        contents: uniform_bytes,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = cell_bind_group(device, "glyph-atlas", layout, &view, sampler, &uniform_buf);
    Ok(GlyphAtlasGpu {
        texture,
        uniform_buf,
        bind_group,
        bytes,
    })
}
