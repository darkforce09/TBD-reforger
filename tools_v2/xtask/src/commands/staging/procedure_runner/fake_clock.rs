//! A test clock that never sleeps: waiting moves its time forward.
//!
//! **Role:** lets the runner's deadlines, request windows and hard stop be tested in
//! microseconds, and lets recorded observers answer by the simulated time.
//!
//! **Position:** test-only; shared by the runner tests and the procedures' tests.
//!
//! **Signals & state:** the simulated Unix time in milliseconds, shared through an `Rc` so a
//! recorded observer reads the same time the runner does.
//!
//! **Invariants:** time only moves forward, by exactly the durations slept or advanced.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use super::clock::Clock;

/// A clock whose `sleep` advances it.
#[derive(Debug, Clone)]
pub(crate) struct FakeClock {
    now_unix_ms: Rc<Cell<u64>>,
}

impl FakeClock {
    /// A clock reading `start_unix_ms`.
    pub(crate) fn starting_at(start_unix_ms: u64) -> Self {
        Self {
            now_unix_ms: Rc::new(Cell::new(start_unix_ms)),
        }
    }

    /// Moves the clock forward by `duration`.
    pub(crate) fn advance(&self, duration: Duration) {
        let step = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
        self.now_unix_ms
            .set(self.now_unix_ms.get().saturating_add(step));
    }
}

impl Clock for FakeClock {
    fn now_unix_ms(&self) -> u64 {
        self.now_unix_ms.get()
    }

    fn sleep(&self, duration: Duration) {
        self.advance(duration);
    }
}
