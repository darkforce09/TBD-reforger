//! The calendar's date arithmetic: local day keys, form values, and the instant they combine to.
//!
//! **Role:** every conversion between the browser's local calendar and the ISO instants the API
//! speaks — the month and weekday captions, the day key the grouping is built on, the values the
//! date and time inputs want, and the comparison the edit form diffs start times with.
//! **Position:** pure helpers under the operations calendar; no view, no request.
//! **Signals & state:** none.
//! **Invariants:** everything here works in the **browser's** zone. A day key is the local
//! calendar day an instant falls on, and `combine_iso` reads a local wall clock back out as an
//! instant, so the grid, the grouping and the form all agree on which day an operation is on and
//! only the string that goes on the wire is UTC. Month numbers are zero-based throughout, matching
//! the JavaScript `Date` they are handed to. These call into `js_sys::Date`, so they answer
//! sensibly only in a browser build.
#![allow(dead_code)]

use wasm_bindgen::JsCast;

/// Month captions for the calendar heading, indexed by zero-based month.
pub(super) const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Column headings of the day grid, starting on Sunday to match `Date::get_day`.
pub(super) const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// `date.toLocaleDateString(undefined, options)`, formatted by the viewer's own locale.
///
/// The method is reached reflectively because the typed binding cannot express the undefined
/// locale argument, and passing an explicit locale would format dates for someone else's
/// conventions. `options` is the plain object of formatting keys the browser expects. Returns an
/// empty string outside a browser, where there is no such method to call.
pub(super) fn locale_date_string(date: &js_sys::Date, options: &[(&str, &str)]) -> String {
    let opts = js_sys::Object::new();
    for (k, v) in options {
        let _ = js_sys::Reflect::set(&opts, &(*k).into(), &(*v).into());
    }
    let f = match js_sys::Reflect::get(date, &"toLocaleDateString".into()) {
        Ok(f) => f,
        Err(_) => return String::new(),
    };
    let f: js_sys::Function = match f.dyn_into() {
        Ok(f) => f,
        Err(_) => return String::new(),
    };
    f.call2(date, &wasm_bindgen::JsValue::UNDEFINED, &opts)
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default()
}

/// The `YYYY-MM-DD` key of a local (year, zero-based month, day) triple.
///
/// Built from the components directly rather than through a formatter, so no time zone is
/// involved and the key cannot slide onto the neighbouring day.
pub(super) fn day_key(y: i32, m0: i32, d: u32) -> String {
    format!("{y:04}-{:02}-{d:02}", m0 + 1)
}

/// The local calendar day an ISO instant falls on, as a [`day_key`].
///
/// An unparseable instant yields an empty key, which matches no cell in the grid.
pub(super) fn iso_day_key(iso: &str) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(iso));
    if d.get_time().is_nan() {
        return String::new();
    }
    day_key(d.get_full_year() as i32, d.get_month() as i32, d.get_date())
}

/// Midnight local time on a (year, zero-based month, day) triple.
pub(super) fn js_date(y: i32, m0: i32, d: u32) -> js_sys::Date {
    js_sys::Date::new_with_year_month_day(y as u32, m0, d as i32)
}

/// A local wall clock — (year, zero-based month, day, hour, minute) — as an ISO instant.
///
/// Publish and edit share it so a rescheduled operation lands on the same instant a freshly
/// published one would.
pub(super) fn combine_iso(y: i32, m0: i32, d: u32, hh: i32, mm: i32) -> String {
    js_sys::Date::new_with_year_month_day_hr_min(y as u32, m0, d as i32, hh, mm)
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}

/// `"HH:MM"` split into hour and minute.
///
/// Lenient by design: an unparseable component reads zero. Callers reject an empty time field
/// before getting here, so the only inputs that reach it are ones the time input produced.
pub(super) fn split_hm(t: &str) -> (i32, i32) {
    t.split_once(':')
        .map(|(h, m)| (h.parse().unwrap_or(0), m.parse().unwrap_or(0)))
        .unwrap_or((0, 0))
}

/// A date input's `"YYYY-MM-DD"` value as (year, zero-based month, day).
///
/// `None` for anything the browser would not have produced, which is how a cleared date field is
/// refused before it can be combined into an instant.
pub(super) fn parse_date_value(s: &str) -> Option<(i32, i32, u32)> {
    let mut parts = s.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: i32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m - 1, d))
}

/// An instant as the local `"YYYY-MM-DD"` a date input wants.
///
/// Empty on an invalid instant, so the field reads blank rather than showing a broken date.
pub(super) fn iso_date_value(iso: &str) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(iso));
    if d.get_time().is_nan() {
        return String::new();
    }
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

/// An instant as the local `"HH:MM"` a time input wants.
pub(super) fn iso_time_value(iso: &str) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(iso));
    if d.get_time().is_nan() {
        return String::new();
    }
    format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
}

/// Whether two ISO strings name the same instant.
///
/// The edit form compares instants rather than strings: a start time read back from the database
/// carries no sub-second part, while the form rebuilds it through the browser's own serializer and
/// gets one. Comparing the text would call an untouched start time "changed" on every save, and a
/// start time in the patch body is not inert — it is the value the server's pre-start guard
/// measures.
pub(super) fn same_instant(a: &str, b: &str) -> bool {
    let (a, b) = (
        js_sys::Date::new(&wasm_bindgen::JsValue::from_str(a)).get_time(),
        js_sys::Date::new(&wasm_bindgen::JsValue::from_str(b)).get_time(),
    );
    !a.is_nan() && !b.is_nan() && a == b
}
