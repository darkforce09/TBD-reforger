//! The date spelling of the JSON wire contract, as one `#[serde(with = …)]` module.
//!
//! **Role:** writes and reads a Postgres `date` field as an instant at midnight UTC.
//! **Position:** applied field by field by the API's models; the instant fields beside it use
//! `fleet_wire_contract::rfc3339_timestamps`, whose spelling this one matches.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a date is written as a full RFC 3339 timestamp (`2026-07-06T00:00:00Z`), never
//! as a bare `2026-07-06`; reading accepts both spellings.

/// A Postgres `date` rendered as an instant at midnight UTC — a full RFC 3339 timestamp
/// (`2026-07-06T00:00:00Z`), NOT a bare `2026-07-06`. Reading accepts both spellings.
pub mod rfc3339_utc_date {
    use chrono::{DateTime, NaiveDate};
    use serde::{Deserialize, Deserializer, Serializer};

    /// Write `date` as `<date>T00:00:00Z`.
    pub fn serialize<S: Serializer>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}T00:00:00Z", date.format("%Y-%m-%d")))
    }

    /// Read an RFC 3339 timestamp as its date, or a bare `YYYY-MM-DD` date.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<NaiveDate, D::Error> {
        let text = String::deserialize(deserializer)?;
        wire_date(&text).map_err(serde::de::Error::custom)
    }

    /// The date `text` spells, as a timestamp's date or as a bare date.
    fn wire_date(text: &str) -> crate::Result<NaiveDate> {
        if let Ok(instant) = DateTime::parse_from_rfc3339(text) {
            return Ok(instant.date_naive());
        }
        Ok(NaiveDate::parse_from_str(text, "%Y-%m-%d")?)
    }
}
