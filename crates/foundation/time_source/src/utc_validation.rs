//! The canonical-UTC rule for stored RFC 3339 timestamps.
//!
//! **Role:** [`validate_rfc3339_utc`]: a value is legal when it parses under the RFC 3339
//! well-known format, its offset is zero, the offset is written `Z` or `+00:00`, and the date and
//! time are separated by an uppercase `T`.
//! **Position:** called by the writers and loaders of lifecycle stamps (`created_at`,
//! `completed_at`); everything [`crate::rfc3339_utc_seconds`] and [`crate::rfc3339_utc_millis`]
//! write passes it.
//! **Signals & state:** none; a pure function over its arguments.
//! **Invariants:** rejected on purpose: naive date-times (no offset, so never RFC 3339), any
//! non-zero offset, `-00:00` ("offset unknown"), and lowercase `z` or `t`. A malformed value is an
//! error for the caller to refuse, never something to replace with the current time.

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::error::{Error, Result};

/// Checks one stored timestamp against the canonical-UTC rule; `field` names the key it came
/// from, and every [`Error`] names it.
pub fn validate_rfc3339_utc(field: &str, value: &str) -> Result<()> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339).map_err(|reason| Error::NotRfc3339 {
        field: field.to_owned(),
        value: value.to_owned(),
        reason: reason.to_string(),
    })?;
    if !parsed.offset().is_utc() {
        return Err(Error::NotUtc {
            field: field.to_owned(),
            value: value.to_owned(),
            offset: parsed.offset().to_string(),
        });
    }
    if !(value.ends_with('Z') || value.ends_with("+00:00")) {
        return Err(Error::NonCanonicalUtcSpelling {
            field: field.to_owned(),
            value: value.to_owned(),
        });
    }
    if value.as_bytes().get(10) != Some(&b'T') {
        return Err(Error::LowercaseSeparator {
            field: field.to_owned(),
            value: value.to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/utc_validation.rs"]
mod tests;
