//! Role: frame atlas.
//! Position: `frame` in the graphics engine.
//! Signals & state: the two cell-atlas textures, their uniform buffers and their bind groups.
//! Invariants: an atlas here is a grid of cells with a uniform block. What a cell depicts —
//! a letter, a unit, a vehicle — is the caller's business and never reaches this module.
//!
//! T-0xx Phase 2B: this was `text/gpu.rs`. It moved for two reasons. `text/` declares
//! "glyph shapes and byte layouts only" and these are neither — they are live GPU handles,
//! and a cell atlas's `bind_group` is precisely what a [`super::TextRun`]'s
//! `atlas: BindGroupId` resolves to, which makes it frame vocabulary. And gate rule 3b bans
//! `website_graphics_engine::text::gpu` from every module of `website-map-engine`: the
//! caller that owns the atlas slot is `RenderEngine`, which did not cross in Phase 1, so the
//! handle type has to be nameable from map-engine. Naming it here puts it behind rule 3a's
//! single seam instead of behind no rule at all.
//!
//! The uniform-block *packing* did not come with it: `TEXT_UNIFORM_BYTES` and
//! `text_uniform_bytes()` touch no `wgpu` type at all, so they belong with the rest of the
//! byte layout in `text::pack` — reachable from map-engine through `crate::layout`.

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
    label: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<(wgpu::Texture, u64), String> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .unwrap_or(0);
    if rgba.len() != expected {
        return Err(format!("{label}-rgba-size"));
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
/// `Err` carries the same tag the caller used to raise — `"text-atlas-rgba-size"` — so the
/// message that reaches the browser is unchanged by the move.
pub fn create_text_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<TextAtlasGpu, String> {
    use wgpu::util::DeviceExt;
    let (texture, bytes) = upload_cell_texture(device, queue, "text-atlas", rgba, width, height)?;
    let u_bytes = crate::text::pack::text_uniform_bytes();
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
) -> Result<GlyphAtlasGpu, String> {
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
