//! Role: lanes.
//! Position: `renderers/text` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::draw_order::LaneRole;
use crate::renderers::batching::batch::Batch;
use crate::renderers::batching::batch::BatchPayload;
use crate::renderers::engine::lifecycle::TEXT_UNIFORM_BYTES;
use wasm_bindgen::prelude::*;

/// Text uniform bytes.
pub(crate) fn text_uniform_bytes() -> [u8; TEXT_UNIFORM_BYTES as usize] {
    let mut u_bytes = [0u8; TEXT_UNIFORM_BYTES as usize];
    u_bytes[0..4].copy_from_slice(&1.0_f32.to_le_bytes());
    #[allow(clippy::cast_precision_loss)]
    let (cols, rows) = (
        crate::renderers::text::atlas::TEXT_ATLAS_COLS as f32,
        crate::renderers::text::atlas::TEXT_ATLAS_ROWS as f32,
    );
    u_bytes[4..8].copy_from_slice(&cols.to_le_bytes());
    u_bytes[8..12].copy_from_slice(&rows.to_le_bytes());
    u_bytes
}

/// Text atlas gpu.
pub(crate) struct TextAtlasGpu {
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
    /// Ensure text atlas.
    pub fn ensure_text_atlas(&mut self) -> Result<(), JsError> {
        if self.text_atlas.is_some() {
            return Ok(());
        }
        let (rgba, w, h) = crate::renderers::text::atlas::bake_ascii_atlas_rgba();
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
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .unwrap_or(0);
        if rgba.len() != expected {
            return Err(JsError::new("text-atlas-rgba-size"));
        }
        use wgpu::util::DeviceExt;
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("text-atlas"),
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
        let u_bytes = text_uniform_bytes();
        let uniform_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("text-uniforms"),
                contents: &u_bytes,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text-atlas"),
            layout: &self.text_bind_group_layout,
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
        if let Some(old) = self.text_atlas.take() {
            old.texture.destroy();
            old.uniform_buf.destroy();
        }
        self.text_atlas = Some(TextAtlasGpu {
            texture,
            uniform_buf,
            bind_group,
            bytes: expected as u64,
        });
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
        self.upsert_lane(
            LaneRole::WorldLabels,
            Batch {
                role: LaneRole::WorldLabels,
                visible: true,
                payload: BatchPayload::IconInstanced {
                    instances: buf,
                    count,
                },
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
        self.upsert_lane(
            LaneRole::WorldTownLabels,
            Batch {
                role: LaneRole::WorldTownLabels,
                visible: true,
                payload: BatchPayload::IconInstanced {
                    instances: buf,
                    count,
                },
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
        self.upsert_lane(
            LaneRole::WorldRoadLabels,
            Batch {
                role: LaneRole::WorldRoadLabels,
                visible: true,
                payload: BatchPayload::IconInstanced {
                    instances: buf,
                    count,
                },
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
