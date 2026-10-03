//! **Role:** [`GlyphAtlasGpu`], the glyph atlas typed layer: the uploaded symbol glyph atlas the
//! world icon lanes sample, and its upload.
//! **Position:** `symbology_layers_gpu`; the renderer holds one as a field, forwards the world
//! loader's atlas upload to [`GlyphAtlasGpu::upload`] with its layer context, and binds
//! [`GlyphAtlasGpu::bind_group`] at the glyph atlas slot of the frame packet.
//! **Signals & state:** the atlas texture, its uniform block and bind group (built by
//! `gpu_frame::frame::create_glyph_atlas`), and the icon bind-group layout and sampler it binds.
//! **Invariants:** an upload replaces the atlas whole and destroys the texture and buffer it
//! replaces; the UV table never exceeds `ATLAS_GLYPH_COUNT` cells.

use crate::error::{Error, Result};
use crate::icon_uniforms::pack_icon_uniforms;
use gpu_frame::frame::GlyphAtlasGpu as UploadedGlyphAtlas;
use renderer_core::layer_context::LayerContext;

/// The glyph atlas the world icon lanes sample.
pub struct GlyphAtlasGpu {
    /// The uploaded atlas, once uploaded.
    uploaded: Option<UploadedGlyphAtlas>,

    /// The icon bind-group layout the atlas bind group is built against.
    icon_bind_group_layout: wgpu::BindGroupLayout,

    /// The sampler the atlas bind group binds.
    icon_sampler: wgpu::Sampler,
}

impl GlyphAtlasGpu {
    /// No atlas yet; the atlas bind group is built against `icon_bind_group_layout` with
    /// `icon_sampler`.
    #[must_use]
    pub fn new(icon_bind_group_layout: wgpu::BindGroupLayout, icon_sampler: wgpu::Sampler) -> Self {
        Self {
            uploaded: None,
            icon_bind_group_layout,
            icon_sampler,
        }
    }

    /// Upload the `width`×`height` RGBA atlas with its UV table `uv` (four `f32` per cell),
    /// replacing the atlas in place.
    ///
    /// # Errors
    /// [`Error::GlyphAtlasUvCount`] when `uv` holds more cells than the uniform block, and
    /// [`Error::GlyphAtlasBuild`] when `rgba` is not `width`×`height`×4 bytes.
    pub fn upload(
        &mut self,
        context: &LayerContext<'_>,
        rgba: &[u8],
        width: u32,
        height: u32,
        uv: &[f32],
    ) -> Result<()> {
        use render_primitives::draw::instances::ATLAS_GLYPH_COUNT;

        if uv.len() > ATLAS_GLYPH_COUNT * 4 {
            return Err(Error::GlyphAtlasUvCount {
                capacity: ATLAS_GLYPH_COUNT * 4,
                actual: uv.len(),
            });
        }
        let u_bytes = pack_icon_uniforms(uv, 0.0, 0.0, 1.0);
        let atlas = gpu_frame::frame::create_glyph_atlas(
            context.device(),
            context.queue(),
            &self.icon_bind_group_layout,
            &self.icon_sampler,
            rgba,
            width,
            height,
            &u_bytes,
        )?;
        if let Some(old) = self.uploaded.take() {
            old.texture.destroy();
            old.uniform_buf.destroy();
        }
        self.uploaded = Some(atlas);
        Ok(())
    }

    /// Whether an atlas is uploaded.
    #[must_use]
    pub fn is_uploaded(&self) -> bool {
        self.uploaded.is_some()
    }

    /// The bind group the glyph-atlas sprite lanes sample, once uploaded.
    #[must_use]
    pub fn bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.uploaded.as_ref().map(|a| &a.bind_group)
    }

    /// The atlas texture's byte size, zero before an upload.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.uploaded.as_ref().map_or(0, |a| a.bytes)
    }
}
