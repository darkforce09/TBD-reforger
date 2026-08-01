//! T-160 — the async GPU readback lifecycle, as a pure state machine.
//!
//! `wgpu::Buffer::map_async` is a two-phase operation with a hard invariant on each side, and
//! **both phases were previously expressed inline in a `wasm32`-only closure inside
//! [`crate::RenderEngine`], where no test can reach them**:
//!
//!   * **Before the map** — a buffer may hold at most one outstanding mapping. A second
//!     `map_async` over a buffer that is already mapped (or already has a map pending) is not a
//!     recoverable error; wgpu answers `Buffer is already mapped` and the frame dies. Something
//!     therefore has to hold "a map is outstanding", and that something has to be consulted by
//!     **every** path that maps, not by one privileged caller who remembers to.
//!   * **After the callback** — the buffer is mapped **iff** the callback's `Result` is `Ok`.
//!     `unmap()` belongs on that arm and only that arm; the in-flight flag has to clear on both,
//!     or one failed readback stops the lane forever.
//!
//! Those two rules are the whole of this module, and they are here rather than in `engine.rs` for
//! one reason: `engine.rs` is `#[cfg(target_arch = "wasm32")]`, so a claim made there is a claim
//! no `cargo test` can check. The GPU stays in `engine.rs`; the *decisions* live here, where the
//! error arm is an ordinary function call.
//!
//! ── What T-160 actually found ────────────────────────────────────────────────────────────────
//!
//! `GpuTimer::kick_readback` mapped **unconditionally**. Its guard lived at its one call site
//! (`render()`'s `take_timing = !t.in_flight.get()`), so the invariant was upheld by the caller's
//! good manners rather than by the function — while the sibling readback
//! (`icon_cull_gpu::kick_readback`) guarded itself and its comment claimed this one did too. That
//! is a latent double-map, not a live one: the editor calls `disable_frame_timing()` and the lane
//! is dormant. It goes live the moment a second caller appears, and the second caller is the
//! fps/GPU-time HUD this timer exists to feed.
//!
//! It also left a **stale sample readable as a fresh one**. On the error arm the old callback
//! cleared `in_flight` and touched nothing else, so `has_sample` stayed `true` over a reading from
//! whichever earlier frame last succeeded — and `stats()` prints `gpu_frame_ms` from exactly that
//! pair. A HUD showing a plausible 2.7 ms for a readback that failed is worse than one showing
//! `null`: `null` is a state the reader can see, and a frozen number is not. [`ReadbackLane`]
//! therefore drops the sample on the error arm — the reader is told the truth, and the next
//! successful frame puts the number back.

use std::cell::Cell;

/// One buffer's `map_async` lifecycle: at most one outstanding mapping, plus whether the value
/// last read out of it is still trustworthy.
///
/// Shared with the `'static` `map_async` callback through an `Rc`, which is why the interior state
/// is `Cell` and every method takes `&self`: on wasm the callback runs on the same thread that
/// armed it, so there is nothing to synchronize and nothing to lock.
#[derive(Debug, Default)]
pub struct ReadbackLane {
    in_flight: Cell<bool>,
    has_sample: Cell<bool>,
}

impl ReadbackLane {
    /// A lane with no mapping outstanding and nothing read yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Claim the lane for one `map_async`.
    ///
    /// `true` — the caller may map, and the lane is now in flight.
    /// `false` — a mapping is already outstanding and the caller **must not** map. This is the
    /// whole guard: it belongs to the lane rather than to any one caller, so a second call site
    /// (the HUD) cannot reintroduce the double-map by forgetting a check it never knew about.
    pub fn begin(&self) -> bool {
        if self.in_flight.get() {
            return false;
        }
        self.in_flight.set(true);
        true
    }

    /// Settle a completed `map_async` callback. `mapped_ok` is `res.is_ok()`.
    ///
    /// Returns **whether the buffer is mapped and the caller must `unmap()` it** — `true` only on
    /// the success arm, because on failure wgpu never mapped it and `unmap()` would be a call
    /// against a buffer in the wrong state.
    ///
    /// Clears in-flight on **both** arms (a lane that stopped rearming after one failed readback
    /// would silently never time another frame), and drops the sample on the failure arm so the
    /// last good reading cannot be served as this frame's.
    pub fn settle(&self, mapped_ok: bool) -> bool {
        self.in_flight.set(false);
        if !mapped_ok {
            self.has_sample.set(false);
        }
        mapped_ok
    }

    /// Record that a value was read out of the mapped range — call from inside the success arm,
    /// after the read. Separate from [`Self::settle`] because settling is about the *mapping* and
    /// this is about the *value*: a callback that maps successfully and then declines to read
    /// (a short buffer, a shape it does not recognise) has no sample to offer.
    pub fn record_sample(&self) {
        self.has_sample.set(true);
    }

    /// Is a mapping outstanding? The render loop asks before encoding a resolve/copy into the
    /// buffer, since writing to a mapped buffer is its own validation error.
    #[must_use]
    pub fn in_flight(&self) -> bool {
        self.in_flight.get()
    }

    /// Is the value last read out of this lane this-frame-fresh? `false` after a failed readback,
    /// so a HUD renders "no reading" rather than a number from an earlier frame.
    #[must_use]
    pub fn has_sample(&self) -> bool {
        self.has_sample.get()
    }
}

#[cfg(test)]
mod tests {
    use super::ReadbackLane;

    /// The double-map, which is the crash T-160 is about. wgpu answers `Buffer is already mapped`
    /// and there is no recovering from it, so the second `map_async` must never be issued.
    ///
    /// RED: delete the `if self.in_flight.get() { return false; }` early-out from
    /// [`ReadbackLane::begin`] — the second claim starts returning `true` and this fails.
    #[test]
    fn a_second_claim_while_one_is_outstanding_is_refused() {
        let lane = ReadbackLane::new();
        assert!(lane.begin(), "the first claim always takes the lane");
        assert!(lane.in_flight());
        assert!(
            !lane.begin(),
            "a second map_async over a still-mapped buffer is the wgpu panic"
        );
        assert!(
            !lane.begin(),
            "and it stays refused — repeated frames must not each get a chance to crash"
        );
    }

    /// The error arm, exercised — the whole point of pulling this out of the wasm-only closure.
    ///
    /// Two claims in one assertion: the caller is told **not** to unmap (wgpu never mapped the
    /// buffer, so `unmap()` is a call in the wrong state), and the lane rearms anyway (a failed
    /// readback must cost one frame's timing, not all of them).
    #[test]
    fn a_failed_map_does_not_unmap_and_does_not_wedge_the_lane() {
        let lane = ReadbackLane::new();
        assert!(lane.begin());
        assert!(
            !lane.settle(false),
            "the failure arm must not unmap — the buffer was never mapped"
        );
        assert!(
            !lane.in_flight(),
            "a failed readback must still clear the flag"
        );
        assert!(lane.begin(), "…so the next frame can try again");
    }

    /// The success arm, and the asymmetry that makes it a real decision: `settle(true)` is the
    /// ONLY thing that authorises an `unmap()`.
    #[test]
    fn a_successful_map_authorises_exactly_one_unmap() {
        let lane = ReadbackLane::new();
        assert!(lane.begin());
        assert!(lane.settle(true), "the success arm owns the unmap");
        assert!(!lane.in_flight());
        // …and the lane is reusable, one map per frame, forever.
        for _ in 0..3 {
            assert!(lane.begin());
            assert!(lane.settle(true));
        }
    }

    /// The stale-reading defect, stated as behaviour: a lane that read 2.7 ms last frame and
    /// failed this frame must not still be offering 2.7 ms. The number itself lives with the
    /// caller; what this owns is whether the caller is allowed to believe it.
    ///
    /// RED: drop the `if !mapped_ok { self.has_sample.set(false); }` arm from
    /// [`ReadbackLane::settle`] — `has_sample` stays `true` across the failure and this fails.
    #[test]
    fn a_failed_readback_retires_the_previous_sample() {
        let lane = ReadbackLane::new();
        // A good frame.
        assert!(lane.begin());
        assert!(lane.settle(true));
        lane.record_sample();
        assert!(lane.has_sample(), "the good reading is on offer");

        // The next frame's readback fails. The old reading is not this frame's.
        assert!(lane.begin());
        assert!(!lane.settle(false));
        assert!(
            !lane.has_sample(),
            "a stale number presented as a fresh one is worse than no number"
        );

        // And a later good frame puts it back — this is a retirement, not a permanent kill.
        assert!(lane.begin());
        assert!(lane.settle(true));
        lane.record_sample();
        assert!(lane.has_sample());
    }

    /// Mapping successfully and reading nothing is a real state (a callback that does not
    /// recognise the bytes), and it must not advertise a sample. `settle(true)` alone never sets
    /// one — only [`ReadbackLane::record_sample`] does.
    #[test]
    fn settling_ok_without_reading_advertises_nothing() {
        let lane = ReadbackLane::new();
        assert!(lane.begin());
        assert!(lane.settle(true));
        assert!(!lane.has_sample());
    }
}
