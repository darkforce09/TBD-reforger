//! Role: gpu.
//! Position: `overlay/symbology/atlas` in the map engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::frame::engine::RenderEngine;

use wasm_bindgen::prelude::*;

/// Re-export `crate::frame::GlyphAtlasGpu`.
// The cell-atlas texture, uniform buffer and bind-group build live in
// `graphics_engine`'s `frame::atlas`: a live atlas is a GPU handle, and its bind group
// is what an icon batch's `BindGroupId` resolves to. `upload_glyph_atlas` lives here: it is a
// `#[wasm_bindgen]` export on this crate's own type, and its uniform block is packed by
// `Self::pack_icon_uniforms`, whose UV table is symbology's cell layout.
pub(crate) use crate::frame::GlyphAtlasGpu;

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
        use render_primitives::draw::instances::ATLAS_GLYPH_COUNT;

        if uv.len() > ATLAS_GLYPH_COUNT * 4 {
            return Err(JsError::new(&format!(
                "glyph-atlas-uv-count: capacity {}, got {}",
                ATLAS_GLYPH_COUNT * 4,
                uv.len()
            )));
        }
        let u_bytes = Self::pack_icon_uniforms(uv, 0.0, 0.0, 1.0);
        let atlas = crate::frame::create_glyph_atlas(
            &self.device,
            &self.queue,
            &self.icon_bind_group_layout,
            &self.icon_sampler,
            rgba,
            width,
            height,
            &u_bytes,
        )
        .map_err(|e| JsError::new(&e))?;
        if let Some(old) = self.glyph_atlas.take() {
            old.texture.destroy();
            old.uniform_buf.destroy();
        }
        self.glyph_atlas = Some(atlas);
        Ok(())
    }
}
