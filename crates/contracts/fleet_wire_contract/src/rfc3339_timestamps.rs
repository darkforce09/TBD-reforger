//! The instant spelling of the fleet wire contract, as two `#[serde(with = …)]` modules.
//!
//! **Role:** writes and reads every timestamp field of the fleet shapes.
//! **Position:** applied field by field by [`crate::operator_messages`] and
//! [`crate::executor_messages`]; the API writes through it and the host agent reads through it.
//! **Signals & state:** none; pure functions.
//! **Invariants:** an instant is written in UTC with a `Z` suffix, fractional seconds only when
//! non-zero and trimmed of trailing zeros (`.5`, not `.500`; `.123456789` at full precision);
//! reading accepts any RFC 3339 offset and converts it to UTC.

/// A required instant as RFC 3339 in UTC.
pub mod rfc3339_utc {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    /// Format a UTC instant in the wire spelling described on [`crate::rfc3339_timestamps`].
    pub fn format(instant: &DateTime<Utc>) -> String {
        let nanos = instant.timestamp_subsec_nanos();
        let base = instant.format("%Y-%m-%dT%H:%M:%S");
        if nanos == 0 {
            format!("{base}Z")
        } else {
            let mut fraction = format!("{nanos:09}");
            while fraction.ends_with('0') {
                fraction.pop();
            }
            format!("{base}.{fraction}Z")
        }
    }

    /// Write `instant` as its [`format()`] string.
    pub fn serialize<S: Serializer>(
        instant: &DateTime<Utc>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format(instant))
    }

    /// Read an RFC 3339 string with any offset as a UTC instant.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<DateTime<Utc>, D::Error> {
        let text = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&text)
            .map(|instant| instant.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}

/// An optional instant in the spelling of [`rfc3339_utc`]; the field's `skip_serializing_if`
/// turns `None` into an absent key, and its `default` reads an absent key as `None`.
pub mod rfc3339_utc_opt {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    /// Write `Some` as its [`super::rfc3339_utc::format`] string and `None` as null.
    pub fn serialize<S: Serializer>(
        instant: &Option<DateTime<Utc>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match instant {
            Some(instant) => serializer.serialize_str(&super::rfc3339_utc::format(instant)),
            None => serializer.serialize_none(),
        }
    }

    /// Read null as `None` and an RFC 3339 string with any offset as a UTC instant.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<DateTime<Utc>>, D::Error> {
        match Option::<String>::deserialize(deserializer)? {
            Some(text) => DateTime::parse_from_rfc3339(&text)
                .map(|instant| Some(instant.with_timezone(&Utc)))
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
#[path = "tests/rfc3339_timestamps.rs"]
mod tests;
