//! Role: bridge 2.
//! Position: `symbology/instances` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::draw_order::LaneRole;
use crate::symbology::instances::bridge_1::SlotAtlasGpu;
use crate::symbology::instances::bridge_1::SlotGpuBridge;
use crate::symbology::instances::drag::pack_drag_overlay;
use crate::symbology::instances::lanes::ICON_DRAG_OFF;
use crate::symbology::instances::lanes::ICON_PXM_OFF;
use crate::symbology::instances::patches::hide_slot_row_patch;
use crate::symbology::instances::patches::pack_selection_only;
use crate::symbology::instances::patches::selected_mask;
use crate::symbology::instances::symbols::SLOT_ICON_STRIDE;
use crate::symbology::instances::symbols::pack_slot_instances;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl RenderEngine {
    /// Whether cluster mode is active for the cached SoA + current zoom.
    #[wasm_bindgen(js_name = cluster_mode)]
    pub fn slots_cluster_mode(&self) -> bool {
        #[allow(clippy::cast_possible_truncation)]
        let n = self.slot_bridge.last_ids.len() as u32;
        crate::symbology::instances::symbols::cluster_mode(n, self.zoom())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Debug JSON for `window.__wgpuSlotStats` (engine stats + bridge flags).
    pub fn slot_stats_json(&self) -> String {
        let stats = self.stats();
        let trimmed = stats.trim_end_matches('}');
        format!(
            "{trimmed},\"slot_len\":{},\"cluster_mode\":{},\"slots_lane_selection_only\":{},\"drag_active\":{},\"atlas_ready\":{}}}",
            self.slot_bridge.last_ids.len(),
            if self.slot_bridge.last_cluster_mode {
                "true"
            } else {
                "false"
            },
            if self.slot_bridge.slots_lane_selection_only {
                "true"
            } else {
                "false"
            },
            if self.slot_bridge.drag_active {
                "true"
            } else {
                "false"
            },
            if self.slot_bridge.atlas_ready {
                "true"
            } else {
                "false"
            },
        )
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Drop slot/drag/cluster lanes and reset bridge SoA cache (keeps atlas).
    pub fn clear_slots(&mut self) {
        self.clear_slot_lanes();
        let atlas_ready = self.slot_bridge.atlas_ready;

        let symbology_base = self.slot_bridge.symbology_base;
        let last_symbology_detailed = self.slot_bridge.last_symbology_detailed;
        self.slot_bridge = SlotGpuBridge {
            atlas_ready,
            symbology_base,
            last_symbology_detailed,
            ..SlotGpuBridge::default()
        };
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload slot atlas.
    pub(crate) fn upload_slot_atlas(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        uv: &[f32],
    ) -> Result<(), JsError> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .unwrap_or(0);
        if rgba.len() != expected {
            return Err(JsError::new("slot-atlas-rgba-size"));
        }
        use wgpu::util::DeviceExt;
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("slot-atlas"),
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

        let px_to_m = 4.0_f32;
        let base_bytes = Self::pack_icon_uniforms(uv, 0.0, 0.0, px_to_m);
        let drag_bytes = Self::pack_icon_uniforms(uv, 0.0, 0.0, px_to_m);
        let base_uniform_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("slot-atlas-base-u"),
                contents: &base_bytes,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let drag_uniform_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("slot-atlas-drag-u"),
                contents: &drag_bytes,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let base_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("slot-atlas-base"),
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
                    resource: base_uniform_buf.as_entire_binding(),
                },
            ],
        });
        let drag_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("slot-atlas-drag"),
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
                    resource: drag_uniform_buf.as_entire_binding(),
                },
            ],
        });
        if let Some(old) = self.slot_atlas.take() {
            old.texture.destroy();
            old.base_uniform_buf.destroy();
            old.drag_uniform_buf.destroy();
        }
        self.slot_atlas = Some(SlotAtlasGpu {
            texture,
            base_uniform_buf,
            drag_uniform_buf,
            base_bind_group,
            drag_bind_group,
            bytes: expected as u64,
            px_to_m,
            drag_delta: [0.0, 0.0],
        });
        Ok(())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Sync slot zoom uniform.
    pub(crate) fn sync_slot_zoom_uniform(&mut self) {
        let px = crate::symbology::instances::symbols::px_to_m_at_zoom(self.zoom());
        self.set_slot_px_to_m(px);

        let detailed = self.symbology_detailed();
        if detailed == self.slot_bridge.last_symbology_detailed {
            return;
        }
        self.slot_bridge.last_symbology_detailed = detailed;
        if self.slot_bridge.symbology_base.is_none() {
            return;
        }
        if !self.slot_bridge.drag_active {
            self.rematerialize_slot_lane();
        }
        self.refresh_comment_lane();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Rematerialize slot lane.
    pub(crate) fn rematerialize_slot_lane(&mut self) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let mask = selected_mask(&self.slot_bridge.last_ids, &self.slot_bridge.selected_ids);
        let zoom = self.zoom();
        #[allow(clippy::cast_possible_truncation)]
        let n = self.slot_bridge.last_ids.len() as u32;
        let cm = crate::symbology::instances::symbols::cluster_mode(n, zoom);
        self.slot_bridge.last_cluster_mode = cm;
        if cm {
            let bytes = pack_selection_only(&self.slot_bridge.last_xy, &mask);
            let vis = !bytes.is_empty();
            self.upload_slot_lane(&bytes, vis);
            self.slot_bridge.slots_lane_selection_only = true;
        } else {
            let bytes = match self.slot_bridge.symbology_base {
                Some(base) => crate::symbology::instances::symbols::pack_slot_symbology(
                    &self.slot_bridge.last_xy,
                    &mask,
                    &self.slot_bridge.last_side_tints,
                    &self.slot_bridge.last_roles,
                    &self.slot_bridge.last_headings,
                    self.slot_m_per_px(),
                    base,
                ),
                None => pack_slot_instances(
                    &self.slot_bridge.last_xy,
                    &mask,
                    &self.slot_bridge.last_side_tints,
                ),
            };
            let vis = !self.slot_bridge.last_ids.is_empty();
            self.upload_slot_lane(&bytes, vis);
            self.slot_bridge.slots_lane_selection_only = false;
        }
        self.slot_bridge.last_symbology_detailed = self.symbology_detailed();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Start slot drag overlay.
    pub(crate) fn start_slot_drag_overlay(&mut self, dx: f32, dy: f32) {
        let drag_ids = self.slot_bridge.drag_ids.clone();
        if drag_ids.is_empty() {
            self.clear_slot_drag_internal();
            return;
        }
        self.slot_bridge.drag_active = true;

        let (overlay, rows) = match self.slot_bridge.symbology_base {
            Some(base) => crate::symbology::instances::drag::pack_drag_overlay_symbology(
                &drag_ids,
                &self.slot_bridge.last_ids,
                &self.slot_bridge.last_xy,
                &self.slot_bridge.last_roles,
                &self.slot_bridge.last_headings,
                self.slot_m_per_px(),
                base,
            ),
            None => pack_drag_overlay(
                &drag_ids,
                &self.slot_bridge.last_ids,
                &self.slot_bridge.last_xy,
            ),
        };
        let count = rows.len();
        self.upload_slot_drag_lane(&overlay, count > 0);

        if !self.slot_bridge.slots_lane_selection_only {
            let hide = hide_slot_row_patch();
            for row in rows {
                #[allow(clippy::cast_possible_truncation)]
                let off = (row * SLOT_ICON_STRIDE + 8) as u32;
                self.patch_slot_lane(off, &hide);
            }
        }
        self.set_slot_drag_delta(dx, dy);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Clear slot drag internal.
    pub(crate) fn clear_slot_drag_internal(&mut self) {
        self.slot_bridge.drag_active = false;
        self.slot_bridge.drag_ids.clear();
        self.clear_slot_drag_lane();
        self.rematerialize_slot_lane();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Set slot px to m.
    pub(crate) fn set_slot_px_to_m(&mut self, px_to_m: f32) {
        let Some(atlas) = self.slot_atlas.as_mut() else {
            return;
        };
        if (atlas.px_to_m - px_to_m).abs() < 1e-9 {
            return;
        }
        atlas.px_to_m = px_to_m;

        let bytes = px_to_m.to_le_bytes();
        self.queue
            .write_buffer(&atlas.base_uniform_buf, ICON_PXM_OFF as u64, &bytes);
        self.queue
            .write_buffer(&atlas.drag_uniform_buf, ICON_PXM_OFF as u64, &bytes);

        self.damage.mark();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Set slot drag delta.
    pub(crate) fn set_slot_drag_delta(&mut self, dx: f32, dy: f32) {
        let Some(atlas) = self.slot_atlas.as_mut() else {
            return;
        };
        atlas.drag_delta = [dx, dy];
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&dx.to_le_bytes());
        bytes[4..8].copy_from_slice(&dy.to_le_bytes());
        bytes[8..12].copy_from_slice(&atlas.px_to_m.to_le_bytes());

        self.queue
            .write_buffer(&atlas.drag_uniform_buf, ICON_DRAG_OFF as u64, &bytes);

        self.uniform_bytes_last_frame = 64 + 16;

        self.damage.mark();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload slot lane.
    pub(crate) fn upload_slot_lane(&mut self, bytes: &[u8], visible: bool) {
        self.upload_slot_role_lane(LaneRole::Slots, bytes, visible);
    }
}
