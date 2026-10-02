//! UTC timestamps written as RFC 3339, to the millisecond or to the second.
//!
//! **Role:** [`rfc3339_utc_millis`] and [`iso_from_system_time`] write
//! `2026-07-04T23:43:38.437Z` (what JavaScript's `new Date(ms).toISOString()` writes);
//! [`rfc3339_utc_seconds`] and [`now_utc_rfc3339`] write `2026-08-14T12:34:56Z`, the one stamp
//! the ticket registry's writers store.
//! **Position:** the emitted artifacts of the developer tools stamp the millisecond form; the
//! ticket registry stamps the whole-second form; [`crate::validate_rfc3339_utc`] accepts both.
//! [`now_utc_rfc3339`] reads [`crate::PlatformClock`].
//! **Signals & state:** none; pure functions, except that [`now_utc_rfc3339`] reads the clock.
//! **Invariants:** the date is the proleptic Gregorian calendar date of the instant (Howard
//! Hinnant's civil-from-days algorithm), the time is truncated, never rounded; years are padded
//! to four digits; an instant before the Unix epoch writes the epoch.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::clock::{Clock, PlatformClock};

const SECONDS_PER_DAY: i64 = 86_400;

/// A UTC calendar date and time of day, to the second.
struct CivilTime {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
}

impl CivilTime {
    /// The calendar date and time `unix_seconds` after the epoch.
    fn from_unix_seconds(unix_seconds: u64) -> Self {
        let seconds = i64::try_from(unix_seconds).unwrap_or(i64::MAX);
        let days = seconds.div_euclid(SECONDS_PER_DAY);
        let time_of_day = seconds.rem_euclid(SECONDS_PER_DAY);
        // Civil from days (Howard Hinnant): eras of 400 years, years starting on 1 March.
        let shifted_days = days + 719_468;
        let era = shifted_days.div_euclid(146_097);
        let day_of_era = shifted_days.rem_euclid(146_097);
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let march_based_month = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * march_based_month + 2) / 5 + 1;
        let month = if march_based_month < 10 {
            march_based_month + 3
        } else {
            march_based_month - 9
        };
        let march_year = year_of_era + era * 400;
        let year = if month <= 2 {
            march_year + 1
        } else {
            march_year
        };
        Self {
            year,
            month,
            day,
            hour: time_of_day / 3600,
            minute: (time_of_day % 3600) / 60,
            second: time_of_day % 60,
        }
    }

    /// `YYYY-MM-DDTHH:MM:SS`, without a fraction or an offset.
    fn date_and_time(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// `unix_seconds` plus `millis` written `YYYY-MM-DDTHH:MM:SS.mmmZ`.
fn millis_stamp(unix_seconds: u64, millis: u32) -> String {
    let civil = CivilTime::from_unix_seconds(unix_seconds);
    format!("{}.{millis:03}Z", civil.date_and_time())
}

/// The instant `unix_ms` milliseconds after the epoch, written like `2026-07-04T23:43:38.437Z`.
pub fn rfc3339_utc_millis(unix_ms: u64) -> String {
    let millis = u32::try_from(unix_ms % 1000).unwrap_or_default();
    millis_stamp(unix_ms / 1000, millis)
}

/// `time` written like `2026-07-04T23:43:38.437Z`, as `new Date(ms).toISOString()` writes it; an
/// instant before the epoch writes `1970-01-01T00:00:00.000Z`.
pub fn iso_from_system_time(time: SystemTime) -> String {
    let since_epoch = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    millis_stamp(since_epoch.as_secs(), since_epoch.subsec_millis())
}

/// The instant `unix_seconds` after the epoch, written like `2026-08-14T12:34:56Z`.
pub fn rfc3339_utc_seconds(unix_seconds: u64) -> String {
    format!(
        "{}Z",
        CivilTime::from_unix_seconds(unix_seconds).date_and_time()
    )
}

/// Now, UTC, whole seconds, written like `2026-08-14T12:34:56Z`; always passes
/// [`crate::validate_rfc3339_utc`].
pub fn now_utc_rfc3339() -> String {
    rfc3339_utc_seconds(PlatformClock.now_unix_ms() / 1000)
}

#[cfg(test)]
#[path = "tests/utc_format.rs"]
mod tests;
