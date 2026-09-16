//! Role: lanes.
//! Position: `frame/upload` in the map engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::frame::bindings;
use crate::frame::engine::RenderEngine;
use crate::frame::{DrawBatch, DrawPayload, InstanceBuffer, TextRun};
use crate::overlay::lanes::LaneRole;
use crate::overlay::lanes::lane_id;
use wasm_bindgen::prelude::*;

/// Re-export `website_graphics_engine::layout::pack::text_uniform_bytes`.
// T-0xx Phase 2B (Kind A): the `TextUniforms` block is bytes, so it sits in graphics-engine's
// `text::pack` and arrives through `layout`, the enumerated ABI surface. The live atlas —
// texture, uniform buffer, bind group — is `frame::TextAtlasGpu`: a GPU handle, which is what
// a `TextRun`'s `atlas: BindGroupId` resolves to. The `impl RenderEngine` blocks below stay
// where they are — they are `#[wasm_bindgen]` exports on a type this crate defines, and
// E0116 is symmetric.
pub(crate) use website_graphics_engine::layout::pack::text_uniform_bytes;

/// Re-export `crate::frame::TextAtlasGpu`.
pub(crate) use crate::frame::TextAtlasGpu;

#[wasm_bindgen]
impl RenderEngine {
    /// Ensure text atlas.
    pub fn ensure_text_atlas(&mut self) -> Result<(), JsError> {
        if self.text_atlas.is_some() {
            return Ok(());
        }
        let (rgba, w, h) = crate::overlay::symbology::text_metrics::atlas::bake_ascii_atlas_rgba();
        self.upload_text_atlas(&rgba, w, h)
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload the baked ASCII atlas (`bake_ascii_atlas_rgba` output — grid dims travel in the `TextUniforms`, see [`text_uniform_bytes`]).
    pub fn upload_text_atlas(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), JsError> {
        let atlas = crate::frame::create_text_atlas(
            &self.device,
            &self.queue,
            &self.text_bind_group_layout,
            &self.icon_sampler,
            rgba,
            width,
            height,
        )
        .map_err(|e| JsError::new(&e))?;
        if let Some(old) = self.text_atlas.take() {
            old.texture.destroy();
            old.uniform_buf.destroy();
        }
        self.text_atlas = Some(atlas);
        Ok(())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload height / cartographic text labels (20 B instances, `WorldLabels` lane).
    pub fn upload_text_labels(&mut self, bytes: &[u8], visible: bool) {
        self.text_label_uploads += 1;
        use wgpu::util::DeviceExt;
        const STRIDE: usize = 20;
        const STRIDE_U32: u32 = 20;
        if bytes.is_empty() || !visible {
            self.text_labels_drawn = 0;
            if !visible {
                self.remove_lane(LaneRole::WorldLabels);
            }
            return;
        }
        if !bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::WorldLabels);
            return;
        }
        let _ = self.ensure_text_atlas();
        let mut converted = bytes.to_vec();
        Self::convert_icon_world_to_anchor(&mut converted);
        let count = (converted.len() / STRIDE) as u32;
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("text-labels"),
                contents: &converted,
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.text_labels_drawn = count;
        let lane = lane_id(LaneRole::WorldLabels);
        self.upsert_lane(
            LaneRole::WorldLabels,
            DrawBatch {
                lane,
                visible: true,
                pipeline: bindings::PIPE_TEXT,
                payload: DrawPayload::Text(TextRun {
                    lane,
                    glyphs: InstanceBuffer::whole(buf, STRIDE_U32, count),
                    atlas: bindings::BIND_TEXT_ATLAS,
                    pipeline: bindings::PIPE_TEXT,
                }),
            },
        );
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload town name labels (20 B instances, `WorldTownLabels` lane — above height labels).
    pub fn upload_town_labels(&mut self, bytes: &[u8], visible: bool) {
        self.text_label_uploads += 1;
        use wgpu::util::DeviceExt;
        const STRIDE: usize = 20;
        const STRIDE_U32: u32 = 20;
        if bytes.is_empty() || !visible {
            self.town_labels_drawn = 0;
            self.remove_lane(LaneRole::WorldTownLabels);
            return;
        }
        if !bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::WorldTownLabels);
            return;
        }
        let _ = self.ensure_text_atlas();
        let mut converted = bytes.to_vec();
        Self::convert_icon_world_to_anchor(&mut converted);
        let count = (converted.len() / STRIDE) as u32;
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("town-labels"),
                contents: &converted,
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.town_labels_drawn = count;
        let lane = lane_id(LaneRole::WorldTownLabels);
        self.upsert_lane(
            LaneRole::WorldTownLabels,
            DrawBatch {
                lane,
                visible: true,
                pipeline: bindings::PIPE_TEXT,
                payload: DrawPayload::Text(TextRun {
                    lane,
                    glyphs: InstanceBuffer::whole(buf, STRIDE_U32, count),
                    atlas: bindings::BIND_TEXT_ATLAS,
                    pipeline: bindings::PIPE_TEXT,
                }),
            },
        );
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload road name labels (20 B instances, `WorldRoadLabels` — below town labels).
    pub fn upload_road_labels(&mut self, bytes: &[u8], visible: bool) {
        self.text_label_uploads += 1;
        use wgpu::util::DeviceExt;
        const STRIDE: usize = 20;
        const STRIDE_U32: u32 = 20;
        if bytes.is_empty() || !visible {
            self.road_labels_drawn = 0;
            if !visible {
                self.remove_lane(LaneRole::WorldRoadLabels);
            }
            return;
        }
        if !bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::WorldRoadLabels);
            return;
        }
        let _ = self.ensure_text_atlas();
        let mut converted = bytes.to_vec();
        Self::convert_icon_world_to_anchor(&mut converted);
        let count = (converted.len() / STRIDE) as u32;
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("road-labels"),
                contents: &converted,
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.road_labels_drawn = count;
        let lane = lane_id(LaneRole::WorldRoadLabels);
        self.upsert_lane(
            LaneRole::WorldRoadLabels,
            DrawBatch {
                lane,
                visible: true,
                pipeline: bindings::PIPE_TEXT,
                payload: DrawPayload::Text(TextRun {
                    lane,
                    glyphs: InstanceBuffer::whole(buf, STRIDE_U32, count),
                    atlas: bindings::BIND_TEXT_ATLAS,
                    pipeline: bindings::PIPE_TEXT,
                }),
            },
        );
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Max texture dimension 2d.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn max_texture_dimension_2d(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Read-only. It exists so a half-resolution basemap can be attributed: if this equals [`Self::max_texture_dimension_2d`] the GPU genuinely cannot do better, and if it is larger the device request lost the resolution somewhere.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn adapter_max_texture_dimension_2d(&self) -> u32 {
        self.adapter_max_texture_dimension_2d
    }
}
