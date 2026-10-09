//! The names a caller of the time source imports with `use time_source::prelude::*;`.

#[cfg(target_arch = "wasm32")]
pub use crate::browser_clock::BrowserClock;
pub use crate::clock::{Clock, PlatformClock, wall_clock_ms};
pub use crate::manual_clock::ManualClock;
pub use crate::monotonic::monotonic_ms;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::system_clock::SystemClock;
pub use crate::utc_format::{
    iso_from_system_time, now_utc_rfc3339, rfc3339_utc_millis, rfc3339_utc_seconds,
};
pub use crate::utc_validation::validate_rfc3339_utc;
