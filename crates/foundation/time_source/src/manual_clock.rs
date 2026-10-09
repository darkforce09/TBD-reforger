//! The test clock.
//!
//! **Role:** [`ManualClock`], a wall clock that stands still until the test sets or advances it.
//! **Position:** injected wherever a caller takes a [`Clock`], usually as `Arc<ManualClock>`
//! shared between the code under test and the test that moves time.
//! **Signals & state:** one atomic millisecond reading, written through `&self`.
//! **Invariants:** reads return exactly the last `new`/`set` value plus every `advance` since,
//! wrapping on `u64` overflow like the atomic add it is.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::clock::Clock;

/// A wall clock that moves only when told to.
#[derive(Debug, Default)]
pub struct ManualClock {
    now_unix_ms: AtomicU64,
}

impl ManualClock {
    /// A clock that reads `unix_ms` until it is set or advanced.
    pub const fn new(unix_ms: u64) -> Self {
        Self {
            now_unix_ms: AtomicU64::new(unix_ms),
        }
    }

    /// Makes the clock read `unix_ms`.
    pub fn set(&self, unix_ms: u64) {
        self.now_unix_ms.store(unix_ms, Ordering::Relaxed);
    }

    /// Moves the clock forward by `elapsed_ms`.
    pub fn advance(&self, elapsed_ms: u64) {
        self.now_unix_ms.fetch_add(elapsed_ms, Ordering::Relaxed);
    }
}

impl Clock for ManualClock {
    fn now_unix_ms(&self) -> u64 {
        self.now_unix_ms.load(Ordering::Relaxed)
    }
}
