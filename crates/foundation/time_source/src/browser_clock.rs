//! The browser wall clock.
//!
//! **Role:** `BrowserClock`, `Date.now()` of the JavaScript host.
//! **Position:** compiled on `wasm32` only, where [`crate::PlatformClock`] names it; works in a
//! window and in a worker alike.
//! **Signals & state:** none; reads the host clock.
//! **Invariants:** [`Clock::now_unix_ms_f64`] is `Date.now()` itself, so a caller that read
//! `js_sys::Date::now()` inline reads the same value through it.

#![cfg(target_arch = "wasm32")]

use crate::clock::Clock;

/// `Date.now()` of the JavaScript host.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BrowserClock;

impl Clock for BrowserClock {
    fn now_unix_ms(&self) -> u64 {
        // `Date.now()` is a whole number of milliseconds; the cast saturates a negative value to 0.
        js_sys::Date::now() as u64
    }

    fn now_unix_ms_f64(&self) -> f64 {
        js_sys::Date::now()
    }
}
