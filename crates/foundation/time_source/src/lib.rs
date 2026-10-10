//! Wall-clock and monotonic time, and UTC timestamps in RFC 3339.
//!
//! **Role:** one [`Clock`] trait (wall-clock Unix milliseconds) with `SystemClock` (native),
//! `BrowserClock` (`wasm32`, `Date.now()`), the per-target alias [`PlatformClock`], its reading
//! as the plain function [`wall_clock_ms`], and the test clock [`ManualClock`]; the monotonic
//! frame-timing source [`monotonic_ms`]; the UTC formatters [`rfc3339_utc_millis`] /
//! [`iso_from_system_time`] (`2026-07-04T23:43:38.437Z`) and [`rfc3339_utc_seconds`] /
//! [`now_utc_rfc3339`] (`2026-08-14T12:34:56Z`); and the canonical-UTC check
//! [`validate_rfc3339_utc`].
//! **Position:** foundation tier; `time` (parsing) on every target, `js-sys` and `web-sys` on
//! `wasm32` only. Every `Date.now()`, `performance.now()` and `SystemTime::now()` reading of the
//! library crates comes through here:
//! - durations and budgets read [`monotonic_ms`] (`performance.now()` with the `Date.now()`
//!   fallback): the map renderer's frame timing, the render diagnostics' benchmark, the
//!   viewshed scheduler's default budget clock (a `fn() -> f64` slot) and the validation panel's
//!   debounce;
//! - wall-clock instants read [`wall_clock_ms`] or [`PlatformClock`]: the streaming settle and
//!   ingest frame stamps, the Mission Creator's persistence, tab-lock, warm-session, hover and
//!   viewport readings, and the frontend's countdown and draft ids;
//! - injected clocks take an `Arc<dyn Clock>`: the CRDT undo-group clocks wrap [`PlatformClock`]
//!   (or [`ManualClock`] in tests) in a `yrs::sync::Clock` adapter with a 1 ms floor;
//! - the developer tools format timestamps with the formatters; [`validate_rfc3339_utc`] checks
//!   a stamp, and its [`Error`] text is the message a caller prints.
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
pub use clock::{Clock, PlatformClock, wall_clock_ms};
pub use error::{Error, Result};
pub use manual_clock::ManualClock;
pub use monotonic::monotonic_ms;
#[cfg(not(target_arch = "wasm32"))]
pub use system_clock::SystemClock;
pub use utc_format::{
    iso_from_system_time, now_utc_rfc3339, rfc3339_utc_millis, rfc3339_utc_seconds,
};
pub use utc_validation::validate_rfc3339_utc;
