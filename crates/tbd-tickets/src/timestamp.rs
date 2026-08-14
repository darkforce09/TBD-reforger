//! T-913.1 — RFC 3339 UTC lifecycle stamps (`created_at` / `completed_at`).
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
/// [`validate_rfc3339_utc`]. The one string the T-913.1 writers stamp.
pub fn now_utc_rfc3339() -> String {
    let now = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("zero nanoseconds is always in range");
    now.format(&Rfc3339)
        .expect("a whole-second UTC instant always formats as RFC 3339")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical_utc() {
        for ok in [
            "2026-08-14T10:00:00Z",
            "2026-08-14T10:00:00.123Z",
            "2026-08-14T10:00:00+00:00",
            "1999-12-31T23:59:59Z",
        ] {
            assert!(validate_rfc3339_utc("created_at", ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn rejects_malformed_and_non_utc() {
        for bad in [
            "2026-13-99T25:61:00Z",      // month 13, day 99, hour 25, minute 61
            "2026-08-14 10:00",          // naive — no T, no seconds, no offset
            "2026-08-14T10:00:00",       // no offset
            "2026-08-14T10:00:00+05:00", // non-UTC offset
            "2026-08-14T10:00:00-00:00", // RFC 3339 "offset unknown"
            "2026-08-14t10:00:00Z",      // lowercase separator
            "2026-08-14T10:00:00z",      // lowercase zulu
            "not a date",
            "",
        ] {
            let err = validate_rfc3339_utc("completed_at", bad);
            assert!(err.is_err(), "{bad:?} must be rejected");
            assert!(
                err.unwrap_err().contains("completed_at"),
                "error must name the field for {bad:?}"
            );
        }
    }

    #[test]
    fn now_is_canonical_and_validates() {
        let now = now_utc_rfc3339();
        assert!(now.ends_with('Z'), "writers stamp Zulu: {now}");
        assert_eq!(now.len(), 20, "whole seconds, no subsecond noise: {now}");
        validate_rfc3339_utc("created_at", &now).expect("now must satisfy its own rule");
    }
}
