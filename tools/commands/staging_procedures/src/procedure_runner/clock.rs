//! The runner's time: the workspace's one wall clock, extended with waiting.
//!
//! **Role:** the [`WaitingClock`] seam the runner reads deadlines from and sleeps through. It
//! extends [`time_source::Clock`], the workspace's one clock, with `sleep`, and the live run
//! passes [`time_source::SystemClock`] itself.
//!
//! **Position:** passed to `runner.rs` by `recording.rs`; `staging_dispatch.rs` passes
//! [`time_source::SystemClock`], tests pass `fake_clock.rs`.
//!
//! **Signals & state:** none; the system clock is the process's wall clock.
//!
//! **Invariants:** reading the time is [`time_source::Clock::now_unix_ms`] and nothing else, so
//! this seam never defines a second reading of the wall clock; time is Unix milliseconds, the unit
//! of the rows the deadlines count from; a clock before 1970 reads as 0 rather than failing a run.

use std::time::Duration;

/// A clock the runner can also wait on: time from [`time_source::Clock`], plus a wait a test
/// clock turns into moving its time forward.
pub(crate) trait WaitingClock: time_source::Clock {
    /// Waits `duration`.
    fn sleep(&self, duration: Duration);
}

impl WaitingClock for time_source::SystemClock {
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}
