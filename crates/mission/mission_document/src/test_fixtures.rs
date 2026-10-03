//! Test clocks for the mission document's undo history.
//!
//! **Role:** [`SteppingClock`], a host clock whose every read lies one step past the gesture
//! window, so each local transaction is its own undo step unless an explicit group holds it.
//! **Position:** compiled for this crate's tests and, through the dev-only `test_fixtures`
//! feature, for the tests of crates that drive a document; injected with
//! [`crate::MissionDocCore::with_undo_clock`].
//! **Signals & state:** one atomic millisecond counter per clock.
//! **Invariants:** reads strictly increase by `GESTURE_WINDOW_MS + 1`, starting one step after 0.

use std::sync::atomic::{AtomicU64, Ordering};

/// A clock that advances by one gesture window plus a millisecond on every read.
#[derive(Debug)]
pub struct SteppingClock {
    now_unix_ms: AtomicU64,
    step: u64,
}

impl SteppingClock {
    /// A clock whose every read lies one step past the gesture window after the previous one.
    #[must_use]
    pub fn past_the_gesture_window() -> Self {
        Self {
            now_unix_ms: AtomicU64::new(0),
            step: mission_crdt::undo_groups::GESTURE_WINDOW_MS + 1,
        }
    }
}

impl time_source::Clock for SteppingClock {
    fn now_unix_ms(&self) -> u64 {
        self.now_unix_ms.fetch_add(self.step, Ordering::Relaxed) + self.step
    }
}
