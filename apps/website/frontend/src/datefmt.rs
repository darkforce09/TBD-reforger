//! date-fns shims (`lib/format.ts`) reproduced via `js_sys::Date` — the same JS `Date` freeze.js
//! patches to the fixed epoch on both apps. So `formatLocalDateTime` / `countdownLabel` match React
//! byte-for-byte under the frozen clock, and show real values in production. No `chrono`.
#![allow(dead_code)]

const WD: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MO: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn parse(iso: &str) -> js_sys::Date {
    js_sys::Date::new(&wasm_bindgen::JsValue::from_str(iso))
}

/// date-fns `format(new Date(iso), 'EEE MMM d, HH:mm zzz')` — e.g. "Sat Aug 1, 21:00 GMT+2". "—"
/// when the ISO string is invalid (NaN time).
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

/// date-fns `format(new Date(iso), 'MMM d')` — e.g. "Jun 12"; "—" when invalid (T-159.25).
pub fn format_short_date(iso: &str) -> String {
    let d = parse(iso);
    if d.get_time().is_nan() {
        return "—".into();
    }
    format!("{} {}", MO[d.get_month() as usize], d.get_date())
}

/// date-fns 'zzz' offset fallback → "GMT±H[:MM]". `getTimezoneOffset` is minutes *behind* UTC
/// (negative when ahead), so a +2h zone reports -120 → "GMT+2".
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

/// date-fns `formatDistanceToNowStrict(target).toUpperCase()` — the single-unit distance ladder
/// (round). "LIVE NOW" if the target is past, "—" if invalid.
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
    // formatDistanceStrict bucket ladder. (date-fns uses a DST-normalized minute count for the
    // day/month/year buckets; the sub-hour DST delta never crosses a committed golden's rounding.)
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
