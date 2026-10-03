//! **Role:** the slot atlas: widen and upload it, arm the slot bridge, and keep the atlas's
//! pixels-to-metres uniform in step with the camera's zoom.
//! **Position:** `symbology_layers_gpu::slot_symbology`, methods of [`SlotSymbology`]; the
//! Mission Creator's canvas mount and the mortar map picker call
//! [`SlotSymbology::ensure_slot_atlas`], and the camera hook calls the zoom sync.
//! **Signals & state:** the slot atlas texture and its base and drag uniform blocks.
//! **Invariants:** an upload replaces the atlas whole and destroys the one it replaces; the
//! symbology base is recorded from the widened strip or cleared when the strip cannot be widened;
//! the zoom sync repacks the lanes only when the detailed / disc threshold is crossed.

use super::state::SlotAtlasGpu;
use super::view::SlotSymbology;
use crate::error::{Error, Result};
use crate::icon_uniforms::{ICON_PXM_OFF, pack_icon_uniforms};

impl SlotSymbology<'_> {
    /// Upload the slot and cluster atlas, widened with the unit, vehicle and comment cells when the
    /// strip can be widened, arm the slot bridge and sync the zoom uniform.
    ///
    /// # Errors
    /// [`Error::SlotAtlasPixelLength`] when `rgba` is not `width`×`height`×4 bytes.
    pub fn ensure_slot_atlas(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        uv: &[f32],
    ) -> Result<()> {
        match unit_symbology::symbol_atlas::extend_atlas_with_unit_glyphs(rgba, width, height) {
            Some(wide) => {
                self.upload_slot_atlas(&wide.rgba, wide.width, wide.height, &wide.uv)?;
                self.slot_bridge.symbology_base = Some(wide.base_cells);
            }
            None => {
                self.upload_slot_atlas(rgba, width, height, uv)?;
                self.slot_bridge.symbology_base = None;
            }
        }
        self.slot_bridge.atlas_ready = true;
        self.slot_bridge.last_symbology_detailed = self.symbology_detailed();
        self.sync_slot_zoom_uniform();
        Ok(())
    }

    /// Upload `rgba` as the slot atlas with its base and drag uniform blocks and bind groups,
    /// replacing the atlas in place.
    pub(super) fn upload_slot_atlas(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        uv: &[f32],
    ) -> Result<()> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .unwrap_or(0);
        if rgba.len() != expected {
            return Err(Error::SlotAtlasPixelLength {
                width,
                height,
                actual: rgba.len(),
            });
        }
        use wgpu::util::DeviceExt;
        let context = self.lanes.layer_context();
        let device = context.device();
        let queue = context.queue();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
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

        let px_to_m = 4.0_f32;
        let base_bytes = pack_icon_uniforms(uv, 0.0, 0.0, px_to_m);
        let drag_bytes = pack_icon_uniforms(uv, 0.0, 0.0, px_to_m);
        let base_uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slot-atlas-base-u"),
            contents: &base_bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let drag_uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slot-atlas-drag-u"),
            contents: &drag_bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let base_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("slot-atlas-base"),
            layout: self.icon_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(self.icon_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: base_uniform_buf.as_entire_binding(),
                },
            ],
        });
        let drag_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("slot-atlas-drag"),
            layout: self.icon_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(self.icon_sampler),
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
        *self.slot_atlas = Some(SlotAtlasGpu {
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

    /// Sync slot zoom uniform.
    pub(super) fn sync_slot_zoom_uniform(&mut self) {
        let px = map_draw_lanes::zoom_gates::px_to_m_at_zoom(self.zoom());
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

    /// Set slot px to m.
    pub(super) fn set_slot_px_to_m(&mut self, px_to_m: f32) {
        let Some(atlas) = self.slot_atlas.as_mut() else {
            return;
        };
        if (atlas.px_to_m - px_to_m).abs() < 1e-9 {
            return;
        }
        atlas.px_to_m = px_to_m;

        let bytes = px_to_m.to_le_bytes();
        let queue = self.lanes.layer_context().queue();
        queue.write_buffer(&atlas.base_uniform_buf, ICON_PXM_OFF as u64, &bytes);
        queue.write_buffer(&atlas.drag_uniform_buf, ICON_PXM_OFF as u64, &bytes);

        self.lanes.mark_damage();
    }
}
