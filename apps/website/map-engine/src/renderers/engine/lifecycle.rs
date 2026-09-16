//! Role: lifecycle.
//! Position: `renderers/engine` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::draw_order::LaneRole;
use crate::core::pipeline::draw_order::lane_order;
use crate::core::pipeline::draw_order::lane_role_from_u32;
use crate::diagnostics::timing::gpu::perf_now_ms;
use crate::renderers::batching::batch::Batch;
use crate::renderers::batching::batch::BatchPayload;
use crate::renderers::batching::scene::ANCHOR;
use crate::symbology::instances::symbols::SLOT_ICON_STRIDE;
use wasm_bindgen::prelude::*;

/// Re-export `website_graphics_engine::text::gpu::TEXT_UNIFORM_BYTES`.
// T-0xx Phase 1D: the size of the text atlas's uniform block is the renderer's, and it moved
// with the block. Re-exported here so the bind-group layout in `core/context/device_2.rs`
// keeps its spelling.
pub(crate) use website_graphics_engine::text::gpu::TEXT_UNIFORM_BYTES;

#[wasm_bindgen]
impl RenderEngine {
    /// Mark dirty.
    pub fn mark_dirty(&mut self) {
        self.damage.mark();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Set continuous render.
    pub fn set_continuous_render(&mut self, on: bool) {
        self.damage.set_continuous(on);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Submitted last frame.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn submitted_last_frame(&self) -> bool {
        self.submitted_last_frame
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// `"webgpu"` or `"webgl2"` — the HUD + verify-gate readout.
    #[must_use]
    pub fn backend(&self) -> String {
        self.backend_kind.clone()
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Render.
    pub fn render(&mut self) -> Result<(), JsError> {
        if !self.damage.begin_frame().submit {
            self.uniform_bytes_last_frame = 0;
            self.submitted_last_frame = false;
            return Ok(());
        }
        let frame_t0 = perf_now_ms();

        let mvp = self.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
        self.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&mvp));

        let drag_live = self
            .batches
            .iter()
            .any(|b| b.role == LaneRole::SlotDrag && b.visible);
        self.uniform_bytes_last_frame = if drag_live { 64 + 16 } else { 64 };

        use wgpu::CurrentSurfaceTexture as Cst;
        let frame = match self.surface.get_current_texture() {
            Cst::Success(f) | Cst::Suboptimal(f) => f,
            Cst::Timeout | Cst::Occluded => {
                self.submitted_last_frame = false;
                return Ok(());
            }
            Cst::Outdated | Cst::Lost => {
                self.surface.configure(&self.device, &self.config);
                match self.surface.get_current_texture() {
                    Cst::Success(f) | Cst::Suboptimal(f) => f,
                    other => {
                        return Err(JsError::new(&format!(
                            "surface-acquire-after-reconfigure: {other:?}"
                        )));
                    }
                }
            }
            other => return Err(JsError::new(&format!("surface-acquire: {other:?}"))),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let take_timing = self.timer.as_ref().is_some_and(|t| !t.lane.in_flight());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        let do_compute = self.encode_main_pass(&mut encoder, &view, take_timing);
        if take_timing && let Some(t) = &self.timer {
            encoder.resolve_query_set(&t.query_set, 0..2, &t.resolve_buf, 0);
            encoder.copy_buffer_to_buffer(&t.resolve_buf, 0, &t.read_buf, 0, 16);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        if take_timing && let Some(t) = &self.timer {
            t.kick_readback();
        }

        if do_compute && let Some(cull) = &self.icon_cull {
            cull.kick_readback();
        }
        self.damage.after_submit();
        self.submitted_last_frame = true;
        self.render_cpu_ms_last = perf_now_ms() - frame_t0;
        self.render_cpu_ms_ema = if self.render_cpu_ms_ema == 0.0 {
            self.render_cpu_ms_last
        } else {
            self.render_cpu_ms_ema * 0.9 + self.render_cpu_ms_last * 0.1
        };
        Ok(())
    }
}

impl RenderEngine {
    /// Upsert lane.
    pub(crate) fn upsert_lane(&mut self, role: LaneRole, batch: Batch) {
        self.remove_lane(role);
        let pos = self
            .batches
            .iter()
            .position(|b| lane_order(b.role) > lane_order(role))
            .unwrap_or(self.batches.len());
        self.batches.insert(pos, batch);
        self.damage.mark();
    }
}

impl RenderEngine {
    /// Remove lane.
    pub(crate) fn remove_lane(&mut self, role: LaneRole) {
        let had = self.batches.iter().any(|b| b.role == role);
        self.batches.retain(|b| b.role != role);
        if had {
            self.damage.mark();
        }
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Set place preview.
    pub fn set_place_preview(&mut self, world_x: f32, world_y: f32) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let tint = crate::symbology::instances::packing::pack_rgba_u32([173, 198, 255, 140]);
        let mut b = Vec::with_capacity(SLOT_ICON_STRIDE);
        crate::symbology::instances::packing::pack_icon_instance(
            &mut b,
            world_x,
            world_y,
            crate::symbology::instances::symbols::SLOT_RING_PX,
            crate::symbology::instances::symbols::SLOT_GLYPH_RING,
            tint,
        );
        self.upload_slot_role_lane(LaneRole::SlotPlacePreview, &b, true);
        self.damage.mark();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Clear place preview.
    pub fn clear_place_preview(&mut self) {
        self.remove_lane(LaneRole::SlotPlacePreview);
        self.damage.mark();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload marker composite.
    pub(crate) fn upload_marker_composite(
        &mut self,
        icon_bytes: &mut [u8],
        caption_bytes: &mut [u8],
    ) {
        const STRIDE: usize = SLOT_ICON_STRIDE;
        if icon_bytes.is_empty() || !icon_bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::MissionMarkers);
            return;
        }
        use wgpu::util::DeviceExt;
        Self::convert_icon_world_to_anchor(icon_bytes);
        let icons = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marker-icons"),
                contents: icon_bytes,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        #[allow(clippy::cast_possible_truncation)]
        let icon_count = (icon_bytes.len() / STRIDE) as u32;
        let captions = if caption_bytes.is_empty() || !caption_bytes.len().is_multiple_of(STRIDE) {
            None
        } else {
            Self::convert_icon_world_to_anchor(caption_bytes);
            let buf = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("marker-captions"),
                    contents: caption_bytes,
                    usage: wgpu::BufferUsages::VERTEX,
                });
            #[allow(clippy::cast_possible_truncation)]
            let count = (caption_bytes.len() / STRIDE) as u32;
            Some((buf, count))
        };
        self.upsert_lane(
            LaneRole::MissionMarkers,
            Batch {
                role: LaneRole::MissionMarkers,
                visible: true,
                payload: BatchPayload::MarkerComposite {
                    icons,
                    icon_count,
                    captions,
                },
            },
        );
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Refresh comment lane.
    pub(crate) fn refresh_comment_lane(&mut self) {
        if self.slot_bridge.comment_xy.is_empty() {
            return;
        }
        let xy = std::mem::take(&mut self.slot_bridge.comment_xy);
        self.comments_bind(&xy);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Drop a vector lane by role id (see `upload_polygon_mesh`). Role 7 (marquee) drops the border lane with the fill.
    pub fn clear_vector_lane(&mut self, role: u32) {
        if let Some(r) = lane_role_from_u32(role) {
            self.remove_lane(r);
            if r == LaneRole::Marquee {
                self.remove_lane(LaneRole::MarqueeOutline);
            }
            self.set_vector_stat(r, 0);
        }
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Hide calibration.
    pub fn hide_calibration(&mut self) {
        for b in &mut self.batches {
            if b.role == LaneRole::Calibration {
                b.visible = false;
            }
        }
    }
}
