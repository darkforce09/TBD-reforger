//! The timestamp and date spellings of the JSON wire contract: three `#[serde(with = …)]`
//! modules the models apply field by field, so every instant this API emits is byte-identical to
//! the string the SPA's DTO golden tests and the Enfusion mod parse.

/// A `timestamptz` as RFC 3339 in UTC: `Z` suffix, fractional seconds only when non-zero and
/// with trailing zeros trimmed (`.5`, not `.500`; `.123456789` at full precision).
pub mod rfc3339_utc {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    /// Format a UTC instant in the wire spelling described on the module.
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

/// An optional timestamp — same wire format as [`rfc3339_utc`], with `None` handled by the
/// caller's `skip_serializing_if` so an absent value is an absent key.
pub mod rfc3339_utc_opt {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(opt: &Option<DateTime<Utc>>, s: S) -> Result<S::Ok, S::Error> {
        match opt {
            Some(dt) => s.serialize_str(&super::rfc3339_utc::format(dt)),
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

/// A Postgres `date` rendered as an instant at midnight UTC — a full RFC 3339 timestamp
/// (`2026-07-06T00:00:00Z`), NOT a bare `2026-07-06`. Reading accepts both spellings.
pub mod rfc3339_utc_date {
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
