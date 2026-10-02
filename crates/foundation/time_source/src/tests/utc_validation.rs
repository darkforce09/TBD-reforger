//! Tests of the canonical-UTC rule: the ticket registry's accept and reject cases, the messages
//! each rejection carries, and the agreement with both formatters.

use super::*;
use crate::{now_utc_rfc3339, rfc3339_utc_millis, rfc3339_utc_seconds};

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
fn now_is_canonical_and_validates() {
    let now = now_utc_rfc3339();
    assert!(now.ends_with('Z'), "writers stamp Zulu: {now}");
    assert_eq!(now.len(), 20, "whole seconds, no subsecond noise: {now}");
    validate_rfc3339_utc("created_at", &now).expect("now must satisfy its own rule");
}

#[test]
fn each_rejection_carries_the_registry_message() {
    let non_utc = validate_rfc3339_utc("created_at", "2026-08-14T10:00:00+05:00").unwrap_err();
    assert_eq!(
        non_utc.to_string(),
        "created_at \"2026-08-14T10:00:00+05:00\" must be UTC (offset +05:00:00); \
         write `Z` or `+00:00`"
    );
    let unknown = validate_rfc3339_utc("created_at", "2026-08-14T10:00:00-00:00").unwrap_err();
    assert_eq!(
        unknown.to_string(),
        "created_at \"2026-08-14T10:00:00-00:00\" must write UTC as `Z` or `+00:00` \
         (`-00:00` means offset-unknown and lowercase `z` is non-canonical)"
    );
    let naive = validate_rfc3339_utc("created_at", "2026-08-14T10:00:00").unwrap_err();
    assert!(matches!(naive, Error::NotRfc3339 { .. }), "{naive:?}");
    assert!(
        naive
            .to_string()
            .starts_with("created_at \"2026-08-14T10:00:00\" is not an RFC 3339 date-time: "),
        "{naive}"
    );
}

#[test]
fn lowercase_spellings_are_refused_whichever_check_catches_them() {
    for (bad, accepted_variant) in [
        ("2026-08-14t10:00:00Z", "LowercaseSeparator or NotRfc3339"),
        (
            "2026-08-14T10:00:00z",
            "NonCanonicalUtcSpelling or NotRfc3339",
        ),
    ] {
        let error = validate_rfc3339_utc("stamp", bad).unwrap_err();
        let expected = match error {
            Error::LowercaseSeparator { .. } => bad.as_bytes()[10] == b't',
            Error::NonCanonicalUtcSpelling { .. } => bad.ends_with('z'),
            Error::NotRfc3339 { .. } => true,
            Error::NotUtc { .. } => false,
        };
        assert!(
            expected,
            "{bad:?} gave {error:?}, expected {accepted_variant}"
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
