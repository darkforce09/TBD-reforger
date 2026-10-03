//! **Role:** the engine's shared text cell atlas, which the label lanes and the marker caption run
//! sample: built on first use from the baked ASCII atlas, or uploaded whole.
//! **Position:** the map renderer's upload belts; the label uploads ensure it through
//! `RenderEngine::ensure_text_atlas`, and the slot symbology's marker bind through the
//! `TextAtlasSlot` the engine lends it as its `TextAtlasSupply`.
//! **Signals & state:** `RenderEngine::text_atlas`, borrowed with the device, queue, text
//! bind-group layout and icon sampler as a `TextAtlasSlot`.
//! **Invariants:** an upload replaces the atlas whole and destroys the texture and buffer it
//! replaces; ensuring an existing atlas does nothing.

use crate::engine::RenderEngine;
use crate::error::Result;
use gpu_frame::frame::{TextAtlasGpu, create_text_atlas};
use symbology_layers_gpu::slot_symbology::TextAtlasSupply;

/// The text atlas slot and what building the atlas needs, borrowed apart from the rest of the
/// engine.
pub(crate) struct TextAtlasSlot<'a> {
    /// The device.
    pub(crate) device: &'a wgpu::Device,

    /// The queue.
    pub(crate) queue: &'a wgpu::Queue,

    /// The text bind-group layout the atlas bind group is built against.
    pub(crate) layout: &'a wgpu::BindGroupLayout,

    /// The sampler the atlas bind group binds.
    pub(crate) sampler: &'a wgpu::Sampler,

    /// The atlas, once built.
    pub(crate) atlas: &'a mut Option<TextAtlasGpu>,
}

impl TextAtlasSlot<'_> {
    /// Build the baked ASCII atlas unless one is in place.
    pub(crate) fn ensure(&mut self) -> Result<()> {
        if self.atlas.is_some() {
            return Ok(());
        }
        let (rgba, w, h) = render_primitives::text::atlas::bake_ascii_atlas_rgba();
        self.upload(&rgba, w, h)
    }

    /// Upload `rgba` as the atlas, replacing the one in place.
    pub(crate) fn upload(&mut self, rgba: &[u8], width: u32, height: u32) -> Result<()> {
        let atlas = create_text_atlas(
            self.device,
            self.queue,
            self.layout,
            self.sampler,
            rgba,
            width,
            height,
        )?;
        if let Some(old) = self.atlas.take() {
            old.texture.destroy();
            old.uniform_buf.destroy();
        }
        *self.atlas = Some(atlas);
        Ok(())
    }
}

impl TextAtlasSupply for TextAtlasSlot<'_> {
    fn ensure_text_atlas(&mut self) -> bool {
        self.ensure().is_ok()
    }
}

impl RenderEngine {
    /// The text atlas slot, borrowed apart from the rest of the engine.
    pub(crate) fn text_atlas_slot(&mut self) -> TextAtlasSlot<'_> {
        TextAtlasSlot {
            device: self.gpu.device(),
            queue: self.gpu.queue(),
            layout: &self.text_bind_group_layout,
            sampler: &self.icon_sampler,
            atlas: &mut self.text_atlas,
        }
    }

    /// Build the baked ASCII text atlas unless one is in place.
    ///
    /// # Errors
    /// [`crate::Error::TextAtlas`] when the baked atlas's pixels do not match its size.
    pub fn ensure_text_atlas(&mut self) -> Result<()> {
        self.text_atlas_slot().ensure()
    }

    /// Upload the baked ASCII atlas (`bake_ascii_atlas_rgba` output — grid dims travel in the `TextUniforms`, see [`render_primitives::text::pack::text_uniform_bytes`]).
    ///
    /// # Errors
    /// [`crate::Error::TextAtlas`] when `rgba` is not `width × height × 4` bytes long.
    pub fn upload_text_atlas(&mut self, rgba: &[u8], width: u32, height: u32) -> Result<()> {
        self.text_atlas_slot().upload(rgba, width, height)
    }
}
