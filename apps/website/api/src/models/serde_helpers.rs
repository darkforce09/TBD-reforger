//! Serde helpers reproducing Go's `encoding/json` wire formats (the Encoder
//! contract from the T-145 plan). Applied field-by-field on the model structs so
//! the Rust JSON output matches the Go service under the `≡` equivalence relation.

/// `time.Time` (Postgres `timestamptz`) rendered as Go's `RFC3339Nano`:
/// trailing-zero-trimmed fractional seconds (`.5`, not `.500`), `Z` for UTC.
pub mod go_time {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    /// Format a UTC instant exactly as Go's `time.Time.MarshalJSON` would.
    pub fn format(dt: &DateTime<Utc>) -> String {
        let nanos = dt.timestamp_subsec_nanos();
        let base = dt.format("%Y-%m-%dT%H:%M:%S");
        if nanos == 0 {
            format!("{base}Z")
        } else {
            let mut frac = format!("{nanos:09}");
            while frac.ends_with('0') {
                frac.pop();
            }
            format!("{base}.{frac}Z")
        }
    }

    pub fn serialize<S: Serializer>(dt: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format(dt))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<DateTime<Utc>, D::Error> {
        let s = String::deserialize(d)?;
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}

/// `Option<time.Time>` — same wire format as [`go_time`], `None` handled by the
/// caller's `skip_serializing_if` (mirrors Go `omitempty` on a nil `*time.Time`).
pub mod go_time_opt {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(opt: &Option<DateTime<Utc>>, s: S) -> Result<S::Ok, S::Error> {
        match opt {
            Some(dt) => s.serialize_str(&super::go_time::format(dt)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
        match Option::<String>::deserialize(d)? {
            Some(s) => DateTime::parse_from_rfc3339(&s)
                .map(|dt| Some(dt.with_timezone(&Utc)))
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

/// Postgres `date` rendered as Go renders it: a `time.Time` at midnight UTC, i.e.
/// a full RFC3339 timestamp (`2026-07-06T00:00:00Z`), NOT a bare `2026-07-06`.
pub mod go_date {
    use chrono::{DateTime, NaiveDate};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(d: &NaiveDate, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{}T00:00:00Z", d.format("%Y-%m-%d")))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveDate, D::Error> {
        let s = String::deserialize(d)?;
        if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
            return Ok(dt.date_naive());
        }
        NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(serde::de::Error::custom)
    }
}
