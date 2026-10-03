//! **Role:** lifecycle — damage, render, the lane-role adapters over the lane sink, the clear
//! colour, vector lane clearing and the queue poll.
//! **Position:** the map renderer; the frame pump calls `render` and `poll`, the upload modules
//! write lanes by role through the adapters here, and the statistics report reads a textured
//! lane's record through `tex_lane`.
//! **Signals & state:** the engine's damage flag, batch list, texture records and render
//! statistics.
//! **Invariants:** `render` submits only a damaged frame and records every frame, submitted or
//! skipped, into the render statistics; it acquires its swapchain image through the GPU context
//! only after the damage gate; a lane-role adapter maps the role onto its `LaneId` and writes
//! through the engine's `LaneSink` implementation, which owns the batch-list mutation and its
//! damage marking.

use crate::engine::RenderEngine;
use crate::error::Result;
use gpu_device::Acquired;
use gpu_frame::frame::DrawBatch;
use gpu_frame::frame::present;
use map_coordinates::terrain_frames::ANCHOR;
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use map_draw_lanes::lane_roles::lane_role_from_u32;
use renderer_core::lane_sink::LaneSink;
use world_layers_gpu::textured_lane::TexLane;

impl RenderEngine {
    /// Mark dirty.
    pub fn mark_dirty(&mut self) {
        self.damage.mark();
    }
}

impl RenderEngine {
    /// Set continuous render.
    pub fn set_continuous_render(&mut self, on: bool) {
        self.damage.set_continuous(on);
    }
}

impl RenderEngine {
    /// `"webgpu"` or `"webgl2"` — the HUD + verify-gate readout.
    #[must_use]
    pub fn backend(&self) -> String {
        self.gpu.backend_kind().as_str().to_owned()
    }
}

impl RenderEngine {
    /// Render one frame if something marked it damaged: write the camera uniform, acquire the
    /// swapchain image, encode the main pass, submit and present.
    ///
    /// # Errors
    /// [`crate::Error::Gpu`] when the surface hands out no image, even after one reconfigure.
    pub fn render(&mut self) -> Result<()> {
        if !self.damage.begin_frame().submit {
            self.uniform_bytes_last_frame = 0;
            self.render_stats.record_skipped_frame();
            return Ok(());
        }
        let frame_t0 = time_source::monotonic_ms();

        let mvp = self.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
        self.gpu
            .queue()
            .write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&mvp));

        let drag_lane = lane_id(LaneRole::SlotDrag);
        let drag_live = self
            .batches
            .iter()
            .any(|b| b.lane == drag_lane && b.visible);
        self.uniform_bytes_last_frame = if drag_live { 64 + 16 } else { 64 };

        // The acquire ladder is the GPU context's and the timestamp resolve, the submit and the
        // present are the GPU frame's. They arrive as two calls rather than one because
        // `encode_main_pass` borrows `self` mutably and must run between them.
        let (frame, view) = match self.gpu.acquire()? {
            Acquired::Ready { frame, view } => (frame, view),
            Acquired::Skip => {
                self.render_stats.record_skipped_frame();
                return Ok(());
            }
        };

        let take_timing = self.timer.as_ref().is_some_and(|t| !t.readback_in_flight());
        let mut encoder =
            self.gpu
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("frame"),
                });
        let do_compute = self.encode_main_pass(&mut encoder, &view, take_timing);
        let resolve =
            self.timer
                .as_ref()
                .filter(|_| take_timing)
                .map(|t| present::TimestampResolve {
                    query_set: t.query_set(),
                    resolve_buf: t.resolve_buffer(),
                    read_buf: t.read_buffer(),
                });
        present::submit(self.gpu.queue(), encoder, frame, resolve);
        if take_timing && let Some(t) = &self.timer {
            t.kick_readback();
        }

        if do_compute {
            self.icon_cull.kick_readback();
        }
        self.damage.after_submit();
        self.render_stats
            .record_submitted_frame(time_source::monotonic_ms() - frame_t0);
        Ok(())
    }
}

impl RenderEngine {
    /// Upsert `role`'s lane: `batch` replaces the lane's batch and drops its texture record,
    /// through [`LaneSink::upsert_lane_batch`]. `batch.lane` is `role`'s lane id.
    pub(crate) fn upsert_lane(&mut self, role: LaneRole, batch: DrawBatch) {
        debug_assert_eq!(
            batch.lane,
            lane_id(role),
            "a batch is keyed by its role's lane"
        );
        self.upsert_lane_batch(batch);
    }
}

impl RenderEngine {
    /// Remove `role`'s lane, through [`LaneSink::remove_lane_batch`].
    pub(crate) fn remove_lane(&mut self, role: LaneRole) {
        self.remove_lane_batch(lane_id(role));
    }
}

impl RenderEngine {
    /// The texture record of `role`'s lane, if it is a live textured lane.
    pub(crate) fn tex_lane(&self, role: LaneRole) -> Option<&TexLane> {
        self.lane_texture(lane_id(role))
    }
}

impl RenderEngine {
    /// Set the background the main pass clears to.
    pub fn set_clear_color(&mut self, r: f64, g: f64, b: f64) {
        self.clear_color = wgpu::Color { r, g, b, a: 1.0 };
    }
}

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

impl RenderEngine {
    /// Let the device run the callbacks of finished GPU work (buffer mappings, timestamp
    /// readbacks) without blocking.
    pub fn poll(&self) {
        let _ = self.gpu.device().poll(wgpu::PollType::Poll);
    }
}

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
