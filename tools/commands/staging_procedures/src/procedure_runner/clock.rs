//! The runner's time: the wall clock in a live run, a fake one in tests.
//!
//! **Role:** the [`Clock`] seam the runner reads deadlines from and sleeps through, and the
//! [`SystemClock`] of live runs, which reads `time_source`'s system clock.
//!
//! **Position:** passed to `runner.rs` by `recording.rs`; tests pass `fake_clock.rs`.
//!
//! **Signals & state:** none; the system clock is the process's wall clock.
//!
//! **Invariants:** time is Unix milliseconds, the unit of the rows the deadlines count from; a
//! clock before 1970 reads as 0 rather than failing a run.

use std::time::Duration;

/// Where the runner reads the time and waits.
pub(crate) trait Clock {
    /// Now, in Unix milliseconds.
    fn now_unix_ms(&self) -> u64;
    /// Waits `duration`.
    fn sleep(&self, duration: Duration);
}

/// The wall clock.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now_unix_ms(&self) -> u64 {
        time_source::Clock::now_unix_ms(&time_source::SystemClock)
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}
