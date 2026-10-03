//! The per-frame ingest budget and the apply-frame statistics.
//!
//! **Role:** opens and closes ingest frames, answers whether an open frame has spent
//! `APPLY_BUDGET_MS`, and records the last and longest apply time and the frames over budget.
//! **Position:** `chunk_scheduler`; methods of
//! [`crate::state::ChunkResidency`]; called through the composed world
//! residency by the world loader's ingest loop and the residency tests.
//! **Signals & state:** mutates the open frame's start and the apply-frame counters; closing a
//! frame evicts.
//! **Invariants:** no open frame is never exhausted; closing a frame evicts once and asks the
//! draw buffers for one full rebuild.

use crate::draw_rebuild::DrawRebuild;
use crate::state::ChunkResidency;

/// Per-frame ingest budget, ms (`chunkStore.ts` `APPLY_BUDGET_MS`).
pub const APPLY_BUDGET_MS: f64 = 4.0;

impl ChunkResidency {
    /// Begin ingest frame at.
    pub fn begin_ingest_frame_at(&mut self, now_ms: f64) {
        self.ingest_frame_start_ms = Some(now_ms);
    }
}

impl ChunkResidency {
    /// True once the current ingest frame has consumed the apply budget. `false` when no frame is open (callers may ingest at least one chunk per frame regardless — the JS loop shape).
    #[must_use]
    pub fn ingest_budget_exhausted_at(&self, now_ms: f64) -> bool {
        match self.ingest_frame_start_ms {
            Some(start) => now_ms - start >= APPLY_BUDGET_MS,
            None => false,
        }
    }
}

impl ChunkResidency {
    /// Close the ingest frame at `now_ms`: records stats + evicts via [`Self::end_apply_frame`] and returns its rebuild. [`DrawRebuild::Nothing`] when no frame is open.
    pub fn end_ingest_frame_at(&mut self, now_ms: f64) -> DrawRebuild {
        match self.ingest_frame_start_ms.take() {
            Some(start) => self.end_apply_frame(now_ms - start),
            None => DrawRebuild::Nothing,
        }
    }
}

impl ChunkResidency {
    /// `drainFrame` tail — record the frame's apply stats, then evict once and ask for one full rebuild. `elapsed_ms` is the wall time the caller measured for this frame's ingest loop.
    pub fn end_apply_frame(&mut self, elapsed_ms: f64) -> DrawRebuild {
        self.apply_frames += 1;
        if elapsed_ms > self.max_apply_ms {
            self.max_apply_ms = elapsed_ms;
        }
        if elapsed_ms > APPLY_BUDGET_MS {
            self.frames_over_budget += 1;
        }
        self.apply_budget_ms_last = elapsed_ms;
        self.evict();
        DrawRebuild::AllBuffers
    }
}

impl ChunkResidency {
    /// Frames over budget.
    #[must_use]
    pub fn frames_over_budget(&self) -> u64 {
        self.frames_over_budget
    }
}
