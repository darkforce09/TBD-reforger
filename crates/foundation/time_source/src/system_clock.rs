//! The native wall clock.
//!
//! **Role:** `SystemClock`, the operating system's real-time clock read through
//! `std::time::SystemTime`.
//! **Position:** compiled on every target except `wasm32`, where `SystemTime::now` panics and
//! `BrowserClock` takes its place; [`crate::PlatformClock`] names it natively.
//! **Signals & state:** none; reads the operating system clock.
//! **Invariants:** whole milliseconds, truncated; 0 for a system clock set before 1970.

#![cfg(not(target_arch = "wasm32"))]

use std::time::{SystemTime, UNIX_EPOCH};

use crate::clock::Clock;

/// The operating system's real-time clock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| {
                u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
            })
    }
}
