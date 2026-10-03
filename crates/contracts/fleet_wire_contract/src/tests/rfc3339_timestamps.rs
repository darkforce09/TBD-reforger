//! Instants are written in UTC with `Z` and trimmed fractions, and read from any RFC 3339 offset.

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use super::{rfc3339_utc, rfc3339_utc_opt};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Stamped {
    #[serde(with = "rfc3339_utc")]
    at: DateTime<Utc>,
    #[serde(
        default,
        with = "rfc3339_utc_opt",
        skip_serializing_if = "Option::is_none"
    )]
    until: Option<DateTime<Utc>>,
}

fn instant(nanos: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 23, 12, 0, 30)
        .single()
        .unwrap()
        + chrono::Duration::nanoseconds(i64::from(nanos))
}

#[test]
fn whole_seconds_carry_no_fraction() {
    assert_eq!(rfc3339_utc::format(&instant(0)), "2026-09-23T12:00:30Z");
}

#[test]
fn fractions_are_trimmed_of_trailing_zeros_at_full_precision() {
    assert_eq!(
        rfc3339_utc::format(&instant(500_000_000)),
        "2026-09-23T12:00:30.5Z"
    );
    assert_eq!(
        rfc3339_utc::format(&instant(120_000_000)),
        "2026-09-23T12:00:30.12Z"
    );
    assert_eq!(
        rfc3339_utc::format(&instant(123_456_789)),
        "2026-09-23T12:00:30.123456789Z"
    );
    assert_eq!(
        rfc3339_utc::format(&instant(1)),
        "2026-09-23T12:00:30.000000001Z"
    );
}

#[test]
fn required_and_optional_instants_round_trip_byte_for_byte() {
    let stamped = Stamped {
        at: instant(500_000_000),
        until: Some(instant(0)),
    };
    let golden = r#"{"at":"2026-09-23T12:00:30.5Z","until":"2026-09-23T12:00:30Z"}"#;
    assert_eq!(serde_json::to_string(&stamped).unwrap(), golden);
    assert_eq!(serde_json::from_str::<Stamped>(golden).unwrap(), stamped);
}

#[test]
fn an_absent_optional_instant_is_an_absent_key_and_reads_from_null() {
    let stamped = Stamped {
        at: instant(0),
        until: None,
    };
    let golden = r#"{"at":"2026-09-23T12:00:30Z"}"#;
    assert_eq!(serde_json::to_string(&stamped).unwrap(), golden);
    assert_eq!(serde_json::from_str::<Stamped>(golden).unwrap(), stamped);
    let with_null = r#"{"at":"2026-09-23T12:00:30Z","until":null}"#;
    assert_eq!(serde_json::from_str::<Stamped>(with_null).unwrap(), stamped);
}

#[test]
fn any_offset_reads_as_the_same_utc_instant() {
    let shifted = r#"{"at":"2026-09-23T14:00:30.5+02:00"}"#;
    assert_eq!(
        serde_json::from_str::<Stamped>(shifted).unwrap().at,
        instant(500_000_000)
    );
}

#[test]
fn text_that_is_not_rfc_3339_is_refused() {
    for invalid in [
        r#"{"at":"2026-09-23"}"#,
        r#"{"at":"2026-09-23T12:00:30"}"#,
        r#"{"at":"yesterday"}"#,
        r#"{"at":1790000000}"#,
        r#"{"at":"2026-09-23T12:00:30Z","until":"soon"}"#,
    ] {
        assert!(
            serde_json::from_str::<Stamped>(invalid).is_err(),
            "{invalid}"
        );
    }
}
