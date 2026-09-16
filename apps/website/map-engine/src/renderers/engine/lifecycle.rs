//! Role: lifecycle.
//! Position: `renderers/engine` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::bindings;
use crate::diagnostics::timing::gpu::perf_now_ms;
use crate::overlay::lanes::LaneRole;
use crate::overlay::lanes::lane_id;
use crate::overlay::lanes::lane_role_from_u32;
use crate::overlay::symbology::instances::symbols::SLOT_ICON_STRIDE;
use crate::renderers::batching::scene::ANCHOR;
use crate::world::terrain::satellite::textures::TexLane;
use wasm_bindgen::prelude::*;
use website_graphics_engine::frame::packet;
use website_graphics_engine::frame::present;
use website_graphics_engine::frame::{DrawBatch, DrawPayload, InstanceBuffer};

/// Re-export `website_graphics_engine::layout::pack::TEXT_UNIFORM_BYTES`.
// T-0xx Phase 2B (Kind A): the size of the text atlas's uniform block is byte layout, not a
// GPU resource — it names no `wgpu` type. It moved out of graphics-engine's `text::gpu` into
// `text::pack` and reaches us through `layout`, the enumerated ABI surface. Re-exported here
// so the bind-group layout in `core/context/device_2.rs` keeps its spelling.
pub(crate) use website_graphics_engine::layout::pack::TEXT_UNIFORM_BYTES;

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

        let drag_lane = lane_id(LaneRole::SlotDrag);
        let drag_live = self
            .batches
            .iter()
            .any(|b| b.lane == drag_lane && b.visible);
        self.uniform_bytes_last_frame = if drag_live { 64 + 16 } else { 64 };

        // T-0xx Phase 1D: the acquire ladder, the timestamp resolve, the submit and the
        // present are `website-graphics-engine`'s. They arrive as two calls rather than one
        // because `encode_main_pass` borrows `self` mutably and must run between them.
        let (frame, view) = match present::acquire(&self.surface, &self.device, &self.config)
            .map_err(|e| JsError::new(&e))?
        {
            present::Acquired::Ready { frame, view } => (frame, view),
            present::Acquired::Skip => {
                self.submitted_last_frame = false;
                return Ok(());
            }
        };

        let take_timing = self.timer.as_ref().is_some_and(|t| !t.lane.in_flight());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        let do_compute = self.encode_main_pass(&mut encoder, &view, take_timing);
        let resolve =
            self.timer
                .as_ref()
                .filter(|_| take_timing)
                .map(|t| present::TimestampResolve {
                    query_set: &t.query_set,
                    resolve_buf: &t.resolve_buf,
                    read_buf: &t.read_buf,
                });
        present::submit(&self.queue, encoder, frame, resolve);
        if take_timing && let Some(t) = &self.timer {
            t.kick_readback();
        }

        if do_compute && let Some(cull) = &self.icon_cull {
            cull.kick_readback();
        }
        self.damage.after_submit();
        self.submitted_last_frame = true;
        self.render_cpu_ms_last = perf_now_ms() - frame_t0;
        self.render_cpu_ms_ema =
            present::frame_ms_ema(self.render_cpu_ms_ema, self.render_cpu_ms_last);
        Ok(())
    }
}

impl RenderEngine {
    /// Upsert lane.
    ///
    /// T-0xx Phase 1D: the ordered insert is `website_graphics_engine::frame::packet::upsert`
    /// over `Vec<DrawBatch>` keyed by `LaneId`. Marking the frame damaged stays — damage is
    /// about whether THIS engine needs to redraw, not about the draw list's shape.
    pub(crate) fn upsert_lane(&mut self, role: LaneRole, batch: DrawBatch) {
        let lane = lane_id(role);
        self.tex_lanes.retain(|(l, _)| *l != lane);
        packet::upsert(&mut self.batches, batch);
        self.damage.mark();
    }
}

impl RenderEngine {
    /// Remove lane.
    pub(crate) fn remove_lane(&mut self, role: LaneRole) {
        let lane = lane_id(role);
        self.tex_lanes.retain(|(l, _)| *l != lane);
        if packet::remove(&mut self.batches, lane) {
            self.damage.mark();
        }
    }
}

impl RenderEngine {
    /// Upsert a textured lane: the batch, plus the texture bookkeeping the batch cannot carry.
    pub(crate) fn upsert_textured_lane(
        &mut self,
        role: LaneRole,
        visible: bool,
        instances: wgpu::Buffer,
        tex: TexLane,
    ) {
        let lane = lane_id(role);
        self.upsert_lane(
            role,
            DrawBatch {
                lane,
                visible,
                pipeline: bindings::textured_pipeline_for(role),
                payload: DrawPayload::TexturedRect {
                    instances: InstanceBuffer::whole(instances, 32, 1),
                    texture: bindings::tex_bind_id(lane),
                },
            },
        );
        self.tex_lanes.push((lane, tex));
    }
}

impl RenderEngine {
    /// The texture bookkeeping for `role`'s lane, if it has one.
    pub(crate) fn tex_lane(&self, role: LaneRole) -> Option<&TexLane> {
        let lane = lane_id(role);
        self.tex_lanes
            .iter()
            .find_map(|(l, t)| (*l == lane).then_some(t))
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Set place preview.
    pub fn set_place_preview(&mut self, world_x: f32, world_y: f32) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let tint =
            crate::overlay::symbology::instances::packing::pack_rgba_u32([173, 198, 255, 140]);
        let mut b = Vec::with_capacity(SLOT_ICON_STRIDE);
        crate::overlay::symbology::instances::packing::pack_icon_instance(
            &mut b,
            world_x,
            world_y,
            crate::overlay::symbology::instances::symbols::SLOT_RING_PX,
            crate::overlay::symbology::instances::symbols::SLOT_GLYPH_RING,
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
        #[allow(clippy::cast_possible_truncation)]
        const STRIDE_U32: u32 = SLOT_ICON_STRIDE as u32;
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
        let lane = lane_id(LaneRole::MissionMarkers);
        self.upsert_lane(
            LaneRole::MissionMarkers,
            DrawBatch {
                lane,
                visible: true,
                pipeline: bindings::PIPE_ICON,
                payload: DrawPayload::SpritesWithText {
                    sprites: InstanceBuffer::whole(icons, STRIDE_U32, icon_count),
                    atlas: bindings::sprite_atlas_for(LaneRole::MissionMarkers),
                    text: captions.map(|(buf, count)| website_graphics_engine::frame::TextRun {
                        lane,
                        glyphs: InstanceBuffer::whole(buf, STRIDE_U32, count),
                        atlas: bindings::BIND_TEXT_ATLAS,
                        pipeline: bindings::PIPE_TEXT,
                    }),
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
        let lane = lane_id(LaneRole::Calibration);
        for b in &mut self.batches {
            if b.lane == lane {
                b.visible = false;
            }
        }
    }
}
