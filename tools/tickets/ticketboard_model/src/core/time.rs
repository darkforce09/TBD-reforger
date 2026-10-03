//! Wall-clock labels for the board's status displays.
//!
//! **Role:** [`utc_hms`] formats seconds since the Unix epoch as an explicit `HH:MM:SS UTC`
//! label, and [`utc_hms_now`] labels the current instant.
//! **Position:** `crate::core`; the strict-check model re-exports [`utc_hms`], and the desktop
//! application stamps finished checks and ticket commands with [`utc_hms_now`]. The clock itself
//! is `time_source::SystemClock`.
//! **Signals & state:** none; [`utc_hms_now`] reads the operating system clock.
//! **Invariants:** labels are UTC with no timezone lookup (the registry's own timestamps are
//! UTC); a clock set before 1970 labels as `00:00:00 UTC`.

use time_source::{Clock, SystemClock};

/// `"HH:MM:SS UTC"` from seconds since the Unix epoch — explicit-UTC on purpose:
/// no timezone dependency, and the registry's own timestamps are UTC.
pub fn utc_hms(secs_since_epoch: u64) -> String {
    let h = (secs_since_epoch / 3600) % 24;
    let m = (secs_since_epoch / 60) % 60;
    let s = secs_since_epoch % 60;
    format!("{h:02}:{m:02}:{s:02} UTC")
}

/// [`utc_hms`] of the current wall-clock second (`time_source::SystemClock`, truncated to
/// whole seconds) — the stamp the status banner and the command drawer show.
pub fn utc_hms_now() -> String {
    utc_hms(SystemClock.now_unix_ms() / 1000)
}
