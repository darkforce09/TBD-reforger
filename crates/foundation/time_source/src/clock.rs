//! The wall-clock trait and the clock of the compilation target.
//!
//! **Role:** [`Clock`], the one question every wall-clock reader asks ("how many milliseconds
//! since the Unix epoch is it now?"), [`PlatformClock`], the clock that answers it for real
//! on the target being compiled, and [`wall_clock_ms`], that answer as a plain function.
//! **Position:** implemented by `SystemClock` (native), `BrowserClock`
//! (`wasm32`) and [`crate::ManualClock`] (tests); read by [`crate::now_utc_rfc3339`] and by
//! callers that take an injectable `Arc<dyn Clock>`.
//! **Signals & state:** none; a trait and a re-export.
//! **Invariants:** `Send + Sync`, so an `Arc<dyn Clock>` crosses threads and fits clock traits
//! that require both; the `f64` reading equals the integer reading below `2^53` ms.

/// A wall clock: milliseconds since 1970-01-01T00:00:00Z.
pub trait Clock: Send + Sync {
    /// Milliseconds since the Unix epoch, UTC; 0 for an instant before it.
    fn now_unix_ms(&self) -> u64;

    /// [`Clock::now_unix_ms`] as an `f64`, the unit `Date.now()` answers in.
    fn now_unix_ms_f64(&self) -> f64 {
        self.now_unix_ms() as f64
    }
}

// The real clock of the compilation target, a unit struct usable as a value
// (`PlatformClock.now_unix_ms()`): `SystemClock` natively, `BrowserClock` on `wasm32`.
#[cfg(target_arch = "wasm32")]
pub use crate::browser_clock::BrowserClock as PlatformClock;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::system_clock::SystemClock as PlatformClock;

/// [`PlatformClock`]'s reading as an `f64`: wall-clock milliseconds since the Unix epoch,
/// `Date.now()` itself on `wasm32` and whole `SystemTime` milliseconds natively. A plain
/// `fn() -> f64`, the wall-clock counterpart of [`crate::monotonic_ms`], so it fills
/// function-pointer clock slots and reads in one call where no clock is injected.
#[must_use]
pub fn wall_clock_ms() -> f64 {
    PlatformClock.now_unix_ms_f64()
}

#[cfg(test)]
#[path = "tests/clock.rs"]
mod tests;
