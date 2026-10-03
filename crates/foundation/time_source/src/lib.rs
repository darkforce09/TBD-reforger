//! Wall-clock and monotonic time, and UTC timestamps in RFC 3339.
//!
//! **Role:** one [`Clock`] trait (wall-clock Unix milliseconds) with `SystemClock` (native),
//! `BrowserClock` (`wasm32`, `Date.now()`), the per-target alias [`PlatformClock`] and the
//! test clock [`ManualClock`]; the monotonic frame-timing source [`monotonic_ms`]; the UTC
//! formatters [`rfc3339_utc_millis`] / [`iso_from_system_time`] (`2026-07-04T23:43:38.437Z`) and
//! [`rfc3339_utc_seconds`] / [`now_utc_rfc3339`] (`2026-08-14T12:34:56Z`); and the canonical-UTC
//! check [`validate_rfc3339_utc`].
//! **Position:** foundation tier; `time` (parsing) on every target, `js-sys` and `web-sys` on
//! `wasm32` only. The clocks and formatters it replaces map onto it as follows:
//! - map engine `diagnostics/timing/gpu.rs`: `now_ms` is `BrowserClock.now_unix_ms_f64()`,
//!   `perf_now_ms` is [`monotonic_ms`] (same `performance.now()` with the `Date.now()` fallback);
//! - map engine viewshed scheduler host: [`monotonic_ms`] is a `fn() -> f64`, so it fills
//!   `SchedulerHost::now_ms` and replaces both fallbacks (native `SystemTime`, wasm tick counter)
//!   with a clock that advances on every target; tests keep injecting their own `fn`;
//! - map engine `streaming/host/viewport.rs` and `world_loader/ingest.rs`: the inline
//!   `js_sys::Date::now()` is `BrowserClock.now_unix_ms_f64()`, the same value;
//! - map engine CRDT undo-group clocks: `RealClock` is [`PlatformClock`], `ManualClock` is
//!   [`ManualClock`], and the injected wasm `fn() -> u64` is no longer needed because
//!   `BrowserClock` reads `Date.now()` itself; a `yrs::sync::Clock` adapter over
//!   `Arc<dyn Clock>` keeps their floor of 1 ms;
//! - developer tools `iso_from_system_time` and ticket tools `now_utc_rfc3339` and
//!   `validate_rfc3339_utc`: the same names here, byte-identical output, and an [`Error`] whose
//!   text is the old message.
//!
//! **Signals & state:** none, except [`ManualClock`]'s atomic reading and the native
//! [`monotonic_ms`] origin, fixed by its first call.
//! **Invariants:** a [`Clock`] answers milliseconds since 1970-01-01T00:00:00Z, 0 before it;
//! [`monotonic_ms`] never decreases; both formatters are the proleptic Gregorian UTC date of the
//! instant, truncated (never rounded) to the millisecond or the second.

mod browser_clock;
mod clock;
mod error;
mod manual_clock;
mod monotonic;
pub mod prelude;
mod system_clock;
mod utc_format;
mod utc_validation;

#[cfg(target_arch = "wasm32")]
pub use browser_clock::BrowserClock;
pub use clock::{Clock, PlatformClock};
pub use error::{Error, Result};
pub use manual_clock::ManualClock;
pub use monotonic::monotonic_ms;
#[cfg(not(target_arch = "wasm32"))]
pub use system_clock::SystemClock;
pub use utc_format::{
    iso_from_system_time, now_utc_rfc3339, rfc3339_utc_millis, rfc3339_utc_seconds,
};
pub use utc_validation::validate_rfc3339_utc;
