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
