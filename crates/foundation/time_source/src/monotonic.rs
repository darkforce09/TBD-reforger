//! The monotonic millisecond source for frame timing and work budgets.
//!
//! **Role:** [`monotonic_ms`], milliseconds from an arbitrary origin that never runs backwards,
//! for measuring durations rather than telling the date.
//! **Position:** a plain `fn() -> f64`, so it fills function-pointer clock slots directly;
//! `performance.now()` on `wasm32`, `std::time::Instant` natively.
//! **Signals & state:** natively, a process-wide origin fixed by the first call; on `wasm32`,
//! none (the page's time origin).
//! **Invariants:** non-decreasing within a thread of execution; sub-millisecond resolution where
//! the platform gives it. On `wasm32` without a `window` (a worker), it falls back to
//! `Date.now()`, which advances but is not monotonic.

/// Milliseconds since this process first asked, from `std::time::Instant`.
#[cfg(not(target_arch = "wasm32"))]
pub fn monotonic_ms() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;

    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    ORIGIN.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

/// Milliseconds since the page's time origin: `performance.now()`, or `Date.now()` where the
/// global scope has no `window`.
#[cfg(target_arch = "wasm32")]
pub fn monotonic_ms() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map_or_else(js_sys::Date::now, |performance| performance.now())
}

#[cfg(test)]
#[path = "tests/monotonic.rs"]
mod tests;
