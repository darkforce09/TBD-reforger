//! The frame and lane counters a renderer keeps about itself.
//!
//! **Role:** [`RenderStats`] holds the CPU cost of the last submitted frame and its moving
//! average, whether the last frame was submitted at all, and one count per lane (how many
//! polygons, segments or instances a lane last uploaded), keyed by the opaque `LaneId`.
//! **Position:** the renderer owns one and records every frame into it; a layer writes its lane
//! counts through the layer context; the renderer's statistics report reads it back into the
//! JSON object [`crate::stats_json::StatsJson`] writes.
//! **Signals & state:** the counters, single-threaded, owned by the renderer.
//! **Invariants:** a lane never counted reads zero; the moving average is seeded by the first
//! submitted frame and then weighs each new frame 1 to 9 (`gpu_frame`'s `frame_ms_ema`); a
//! skipped frame changes neither CPU figure.

use render_primitives::frame::ids::LaneId;
use std::collections::BTreeMap;

/// The frame and lane counters of one renderer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderStats {
    render_cpu_ms_last: f64,
    render_cpu_ms_ema: f64,
    submitted_last_frame: bool,
    lane_counts: BTreeMap<LaneId, u32>,
}

impl RenderStats {
    /// No frame yet, every lane at zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a submitted frame whose CPU side took `cpu_ms`, and fold it into the moving
    /// average.
    #[cfg(target_arch = "wasm32")]
    pub fn record_submitted_frame(&mut self, cpu_ms: f64) {
        self.submitted_last_frame = true;
        self.render_cpu_ms_last = cpu_ms;
        self.render_cpu_ms_ema =
            gpu_frame::frame::present::frame_ms_ema(self.render_cpu_ms_ema, cpu_ms);
    }

    /// Record a frame that was not submitted: nothing was damaged, or the swapchain had no
    /// image.
    pub fn record_skipped_frame(&mut self) {
        self.submitted_last_frame = false;
    }

    /// Whether the last frame was submitted.
    #[must_use]
    pub fn submitted_last_frame(&self) -> bool {
        self.submitted_last_frame
    }

    /// The CPU milliseconds of the last submitted frame.
    #[must_use]
    pub fn render_cpu_ms_last(&self) -> f64 {
        self.render_cpu_ms_last
    }

    /// The moving average of the CPU milliseconds per submitted frame.
    #[must_use]
    pub fn render_cpu_ms_ema(&self) -> f64 {
        self.render_cpu_ms_ema
    }

    /// Set the count `lane` reports.
    pub fn set_lane_count(&mut self, lane: LaneId, count: u32) {
        self.lane_counts.insert(lane, count);
    }

    /// The count `lane` last reported, zero when it never did.
    #[must_use]
    pub fn lane_count(&self, lane: LaneId) -> u32 {
        self.lane_counts.get(&lane).copied().unwrap_or(0)
    }
}

#[cfg(test)]
#[path = "tests/render_stats_tests.rs"]
mod tests;
