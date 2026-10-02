//! Date and time formatting, done against the browser's own clock.
//!
//! **Role:** turns the wire's ISO timestamps into the strings the interface shows.
//! **Position:** called at render time by whatever displays a timestamp.
//! **Signals & state:** none, but every function reads the current time zone and, for the
//! countdown, the current time — so two calls a second apart can differ.
//! **Invariants:** parsing goes through the browser's own date object rather than a date library,
//! which keeps the bundle smaller and means a page rendered under a frozen clock produces stable
//! output. An unparseable timestamp always yields a dash rather than a panic or an epoch date.
#![allow(dead_code)]

const WD: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MO: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The browser's date object for an ISO timestamp. An invalid string yields one whose time is
/// not a number, which every caller checks for.
pub(super) fn parse(iso: &str) -> js_sys::Date {
    js_sys::Date::new(&wasm_bindgen::JsValue::from_str(iso))
}

/// A timestamp as weekday, month, day, time and zone — for example "Sat Aug 1, 21:00 GMT+2".
///
/// A dash when the timestamp cannot be parsed.
pub fn format_local_datetime(iso: &str) -> String {
    let d = parse(iso);
    if d.get_time().is_nan() {
        return "—".into();
    }
    format!(
        "{} {} {}, {:02}:{:02} {}",
        WD[d.get_day() as usize],
        MO[d.get_month() as usize],
        d.get_date(),
        d.get_hours(),
        d.get_minutes(),
        tz_label(d.get_timezone_offset()),
    )
}

/// A timestamp as month and day — for example "Jun 12". A dash when it cannot be parsed.
pub fn format_short_date(iso: &str) -> String {
    let d = parse(iso);
    if d.get_time().is_nan() {
        return "—".into();
    }
    format!("{} {}", MO[d.get_month() as usize], d.get_date())
}

/// A time-zone offset as "GMT±H" or "GMT±H:MM".
///
/// The browser reports the offset as minutes *behind* UTC, so a zone two hours ahead reports minus
/// a hundred and twenty — which is why the sign is inverted here.
fn tz_label(offset_min: f64) -> String {
    let sign = if offset_min <= 0.0 { '+' } else { '-' };
    let abs = offset_min.abs();
    let h = (abs / 60.0) as i64;
    let m = (abs % 60.0) as i64;
    if m == 0 {
        format!("GMT{sign}{h}")
    } else {
        format!("GMT{sign}{h}:{m:02}")
    }
}

/// A duration in seconds as zero-padded hours, minutes and seconds.
///
/// Shared by the panels that show how long a server has been up, so they cannot drift apart. The
/// administration screen deliberately uses its own coarser form instead.
pub fn format_uptime(seconds: i64) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

/// A timestamp as a fixed-width stamp in the viewer's zone, for a log column.
///
/// [`format_local_datetime`] is the prose form; a column of log lines wants alignment instead.
pub fn log_stamp(iso: &str) -> String {
    let d = parse(iso);
    if d.get_time().is_nan() {
        return "--------- --:--:--".into();
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date(),
        d.get_hours(),
        d.get_minutes(),
        d.get_seconds()
    )
}
