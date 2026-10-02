//! RFC 3339 UTC lifecycle stamps (`created_at` / `completed_at`).
//!
//! THE RULE (UTC-only, canonical): a value is legal iff it parses under the RFC 3339
//! well-known format, its offset is zero, the offset is WRITTEN `Z` or `+00:00`, and the
//! date/time separator is an uppercase `T`. Rejected on purpose: naive datetimes (no
//! offset — they never parse as RFC 3339), any non-zero offset (`+05:00`), RFC 3339's
//! `-00:00` ("offset unknown"), and lowercase `z`/`t`. A malformed value is a LOAD ERROR
//! that refuses the tree — never silently substituted with now.

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Validate one lifecycle stamp. `field` names the offending key in the error.
pub fn validate_rfc3339_utc(field: &str, value: &str) -> Result<(), String> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|e| format!("{field} {value:?} is not an RFC 3339 date-time: {e}"))?;
    if !parsed.offset().is_utc() {
        return Err(format!(
            "{field} {value:?} must be UTC (offset {}); write `Z` or `+00:00`",
            parsed.offset()
        ));
    }
    if !(value.ends_with('Z') || value.ends_with("+00:00")) {
        return Err(format!(
            "{field} {value:?} must write UTC as `Z` or `+00:00` \
             (`-00:00` means offset-unknown and lowercase `z` is non-canonical)"
        ));
    }
    if value.as_bytes().get(10) != Some(&b'T') {
        return Err(format!(
            "{field} {value:?} must separate date and time with an uppercase `T`"
        ));
    }
    Ok(())
}

/// Now, UTC, whole seconds, rendered `2026-08-14T12:34:56Z` — always passes
/// [`crate::validate_rfc3339_utc`]. The one string the writers stamp.
pub fn now_utc_rfc3339() -> String {
    let now = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("zero nanoseconds is always in range");
    now.format(&Rfc3339)
        .expect("a whole-second UTC instant always formats as RFC 3339")
}

#[cfg(test)]
#[path = "tests/timestamp/mod.rs"]
mod tests;
