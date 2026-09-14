//! The live countdown to a scheduled moment.
//!
//! **Role:** turns the time remaining until a timestamp into the single-unit phrase the interface
//! shows.
//! **Position:** rendered wherever an upcoming operation is listed, and re-evaluated on a tick.
//! **Signals & state:** none, but it reads the current time, so the same argument gives a different
//! answer as time passes.
//! **Invariants:** one unit only, rounded — the largest unit the remaining time reaches. A moment
//! that has passed reads as live rather than as a negative duration, and an unparseable timestamp
//! yields a dash.
#![allow(dead_code)]

use super::datefmt::parse;

/// How long until `iso`, in one rounded unit and upper case — for example "3 HOURS".
///
/// Reads "LIVE NOW" once the moment has passed, and a dash when the timestamp cannot be parsed.
pub fn countdown_label(iso: &str) -> String {
    let target = parse(iso);
    let t = target.get_time();
    if t.is_nan() {
        return "—".into();
    }
    let now = js_sys::Date::now();
    if t <= now {
        return "LIVE NOW".into();
    }
    let ms = t - now;
    let minutes = ms / 60_000.0;
    // The unit ladder, largest unit that the remaining time reaches. The day, month and year
    // buckets ignore daylight-saving shifts: the sub-hour difference never changes the rounding.
    let (val, unit) = if minutes < 1.0 {
        ((ms / 1000.0).round(), "second")
    } else if minutes < 60.0 {
        (minutes.round(), "minute")
    } else if minutes < 1440.0 {
        ((minutes / 60.0).round(), "hour")
    } else if minutes < 43200.0 {
        ((minutes / 1440.0).round(), "day")
    } else if minutes < 525600.0 {
        ((minutes / 43200.0).round(), "month")
    } else {
        ((minutes / 525600.0).round(), "year")
    };
    let val = val as i64;
    let plural = if val == 1 { "" } else { "s" };
    format!("{val} {unit}{plural}").to_uppercase()
}
