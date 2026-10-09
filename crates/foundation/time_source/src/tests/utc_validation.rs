//! Tests of the canonical-UTC rule: the ticket registry's accept and reject cases and the
//! agreement with both formatters.

use super::*;
use crate::{rfc3339_utc_millis, rfc3339_utc_seconds};

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
            err.unwrap_err().to_string().contains("completed_at"),
            "error must name the field for {bad:?}"
        );
    }
}

#[test]
fn both_formatters_write_values_the_rule_accepts() {
    for unix_seconds in [0, 951_782_400, 1_709_208_000, 1_783_208_618, 4_107_542_400] {
        let seconds = rfc3339_utc_seconds(unix_seconds);
        validate_rfc3339_utc("seconds", &seconds).unwrap();
        let millis = rfc3339_utc_millis(unix_seconds * 1000 + 437);
        validate_rfc3339_utc("millis", &millis).unwrap();
    }
}
