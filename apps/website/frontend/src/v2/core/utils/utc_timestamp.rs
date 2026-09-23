//! RFC 3339 UTC timestamps read, compared and written without the browser clock.
//!
//! **Role:** parses the wire's UTC instants into comparable values, renders one as the UTC line
//! shown beside a local time, and converts between an instant and the `YYYY-MM-DDTHH:MM` value a
//! date-and-time field holds when the field is read as UTC.
//! **Position:** called at render time wherever a UTC instant is shown or edited alongside the
//! viewer's local rendering of it.
//! **Signals & state:** none. Every function is pure, so all of it runs in the native tests.
//! **Invariants:** only UTC designators (`Z`, `+00:00`, `-00:00`) are accepted — the backend writes
//! every instant in UTC, and reading another offset as UTC would shift it silently. Comparison is by
//! value, not by text: the wire trims trailing zeros from the fraction, so `…:44.5Z` and `…:44Z`
//! differ in length and a text comparison would order them wrongly.

use std::fmt;

/// One UTC instant, at nanosecond precision.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcTimestamp {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    nanosecond: u32,
}

#[allow(dead_code)]
impl UtcTimestamp {
    /// Read an RFC 3339 instant in UTC, such as `2026-07-15T14:05:44.629713Z`.
    ///
    /// `None` for anything else, including an instant written with a non-zero offset.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let body = text
            .strip_suffix('Z')
            .or_else(|| text.strip_suffix('z'))
            .or_else(|| text.strip_suffix("+00:00"))
            .or_else(|| text.strip_suffix("-00:00"))?;
        let (date, time) = body.split_once(['T', 't'])?;
        let (year, month, day) = parse_date(date)?;
        let (clock, fraction) = match time.split_once('.') {
            Some((clock, fraction)) => (clock, Some(fraction)),
            None => (time, None),
        };
        let mut parts = clock.split(':');
        let hour = parse_number(parts.next()?, 2)?;
        let minute = parse_number(parts.next()?, 2)?;
        let second = parse_number(parts.next()?, 2)?;
        if parts.next().is_some() || hour > 23 || minute > 59 || second > 60 {
            return None;
        }
        let nanosecond = match fraction {
            Some(digits)
                if !digits.is_empty()
                    && digits.len() <= 9
                    && digits.bytes().all(|b| b.is_ascii_digit()) =>
            {
                format!("{digits:0<9}").parse().ok()?
            }
            Some(_) => return None,
            None => 0,
        };
        Some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanosecond,
        })
    }

    /// Read a date-and-time field's `YYYY-MM-DDTHH:MM` value — seconds optional — as UTC.
    pub fn from_datetime_field(value: &str) -> Option<Self> {
        let (date, time) = value.trim().split_once('T')?;
        let (year, month, day) = parse_date(date)?;
        let mut parts = time.split(':');
        let hour = parse_number(parts.next()?, 2)?;
        let minute = parse_number(parts.next()?, 2)?;
        let second = match parts.next() {
            Some(s) => parse_number(s, 2)?,
            None => 0,
        };
        if parts.next().is_some() || hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        Some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanosecond: 0,
        })
    }

    /// The `YYYY-MM-DDTHH:MM` value a date-and-time field shows for this instant, read as UTC.
    pub fn datetime_field_value(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }

    /// The instant in the wire spelling: seconds always, a fraction only when non-zero.
    pub fn to_rfc3339(self) -> String {
        let base = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        );
        if self.nanosecond == 0 {
            format!("{base}Z")
        } else {
            let fraction = format!("{:09}", self.nanosecond);
            format!("{base}.{}Z", fraction.trim_end_matches('0'))
        }
    }
}

/// The UTC line shown beside a local time: `2026-07-15 14:05 UTC`.
impl fmt::Display for UtcTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02} UTC",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }
}

/// The UTC line for a wire instant, or the text itself when it is not a UTC instant, so a value
/// this reader does not understand is still shown rather than hidden.
#[allow(dead_code)]
pub fn utc_label(text: &str) -> String {
    UtcTimestamp::parse(text)
        .map(|instant| instant.to_string())
        .unwrap_or_else(|| text.to_string())
}

/// `YYYY-MM-DD` as a calendar date, with the day checked against its month.
fn parse_date(date: &str) -> Option<(i32, u32, u32)> {
    let mut parts = date.split('-');
    let year = i32::try_from(parse_number(parts.next()?, 4)?).ok()?;
    let month = parse_number(parts.next()?, 2)?;
    let day = parse_number(parts.next()?, 2)?;
    if parts.next().is_some() || !(1..=12).contains(&month) {
        return None;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in_month = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month)
        .contains(&day)
        .then_some((year, month, day))
}

/// A run of exactly `width` ASCII digits.
fn parse_number(text: &str, width: usize) -> Option<u32> {
    (text.len() == width && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

#[cfg(test)]
#[path = "tests/utc_timestamp.rs"]
mod tests;
