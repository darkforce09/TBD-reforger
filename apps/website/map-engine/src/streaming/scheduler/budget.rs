//! Role: budget.
//! Position: `streaming/scheduler` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::streaming::scheduler::state::WorldResidency;

/// Per-frame ingest budget, ms (`chunkStore.ts` `APPLY_BUDGET_MS`).
pub const APPLY_BUDGET_MS: f64 = 4.0;

impl WorldResidency {
    /// Begin ingest frame at.
    pub fn begin_ingest_frame_at(&mut self, now_ms: f64) {
        self.ingest_frame_start_ms = Some(now_ms);
    }
}

impl WorldResidency {
    /// True once the current ingest frame has consumed the apply budget. `false` when no frame is open (callers may ingest at least one chunk per frame regardless — the JS loop shape).
    #[must_use]
    pub fn ingest_budget_exhausted_at(&self, now_ms: f64) -> bool {
        match self.ingest_frame_start_ms {
            Some(start) => now_ms - start >= APPLY_BUDGET_MS,
            None => false,
        }
    }
}

impl WorldResidency {
    /// Close the ingest frame at `now_ms`: records stats + evicts + rebuilds via [`Self::end_apply_frame`]. No-op when no frame is open.
    pub fn end_ingest_frame_at(&mut self, now_ms: f64) {
        if let Some(start) = self.ingest_frame_start_ms.take() {
            self.end_apply_frame(now_ms - start);
        }
    }
}

impl WorldResidency {
    /// `drainFrame` tail — record the frame's apply stats, then evict + rebuild once. `elapsed_ms` is the wall time the caller measured for this frame's ingest loop.
    pub fn end_apply_frame(&mut self, elapsed_ms: f64) {
        self.apply_frames += 1;
        if elapsed_ms > self.max_apply_ms {
            self.max_apply_ms = elapsed_ms;
        }
        if elapsed_ms > APPLY_BUDGET_MS {
            self.frames_over_budget += 1;
        }
        self.apply_budget_ms_last = elapsed_ms;
        self.evict();
        self.rebuild_buffers();
    }
}

impl WorldResidency {
    /// Frames over budget.
    #[must_use]
    pub fn frames_over_budget(&self) -> u64 {
        self.frames_over_budget
    }
}
