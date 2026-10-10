//! Tests of the UTC formatters: pinned instants (epoch, leap days, the documented example),
//! truncation and padding of the milliseconds, the pre-epoch floor, and cross-checks of the
//! millisecond and whole-second forms against each other and against the `time` crate.

use std::time::Duration;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::*;

/// 2026-07-04T23:43:38Z, the example in the millisecond formatter's documentation.
const DOCUMENTED_SECONDS: u64 = 1_783_208_618;

/// The whole-second stamp for `unix_seconds`, written through `time`.
fn time_crate_seconds(unix_seconds: u64) -> String {
    OffsetDateTime::from_unix_timestamp(i64::try_from(unix_seconds).unwrap())
        .unwrap()
        .format(&Rfc3339)
        .unwrap()
}

/// Instants spread over 1970–2100, with every day boundary and leap day of a few years.
fn sweep_seconds() -> Vec<u64> {
    let mut instants: Vec<u64> = (0..4_107_542_400).step_by(7 * 86_400 + 3_601).collect();
    for year_start in [946_684_800_u64, 951_782_400, 1_704_067_200, 4_102_444_800] {
        for day in 0..400 {
            let midnight = year_start + day * 86_400;
            instants.extend([midnight, midnight - 1, midnight + 43_199]);
        }
    }
    instants
}

#[test]
fn writes_the_documented_example() {
    assert_eq!(
        rfc3339_utc_millis(DOCUMENTED_SECONDS * 1000 + 437),
        "2026-07-04T23:43:38.437Z"
    );
    let time = UNIX_EPOCH + Duration::from_millis(DOCUMENTED_SECONDS * 1000 + 437);
    assert_eq!(iso_from_system_time(time), "2026-07-04T23:43:38.437Z");
    assert_eq!(rfc3339_utc_seconds(1_786_710_896), "2026-08-14T12:34:56Z");
}

#[test]
fn writes_the_epoch_and_leap_days() {
    assert_eq!(rfc3339_utc_millis(0), "1970-01-01T00:00:00.000Z");
    assert_eq!(rfc3339_utc_seconds(0), "1970-01-01T00:00:00Z");
    assert_eq!(rfc3339_utc_seconds(951_782_400), "2000-02-29T00:00:00Z");
    assert_eq!(rfc3339_utc_seconds(1_709_208_000), "2024-02-29T12:00:00Z");
    assert_eq!(rfc3339_utc_seconds(946_684_799), "1999-12-31T23:59:59Z");
    assert_eq!(rfc3339_utc_seconds(4_107_542_400), "2100-03-01T00:00:00Z");
}

#[test]
fn milliseconds_are_padded_and_truncated() {
    assert_eq!(rfc3339_utc_millis(7), "1970-01-01T00:00:00.007Z");
    assert_eq!(rfc3339_utc_millis(1_999), "1970-01-01T00:00:01.999Z");
    let almost = UNIX_EPOCH + Duration::from_nanos(1_999_999_999);
    assert_eq!(iso_from_system_time(almost), "1970-01-01T00:00:01.999Z");
}

#[test]
fn an_instant_before_the_epoch_writes_the_epoch() {
    let before = UNIX_EPOCH - Duration::from_secs(86_400);
    assert_eq!(iso_from_system_time(before), "1970-01-01T00:00:00.000Z");
}

#[test]
fn the_whole_second_form_matches_the_time_crate() {
    for unix_seconds in sweep_seconds() {
        assert_eq!(
            rfc3339_utc_seconds(unix_seconds),
            time_crate_seconds(unix_seconds),
            "{unix_seconds}"
        );
    }
}

#[test]
fn the_millisecond_form_is_the_second_form_plus_a_fraction() {
    for unix_seconds in sweep_seconds() {
        for millis in [0, 7, 437, 999] {
            let unix_ms = unix_seconds * 1000 + millis;
            let with_fraction = rfc3339_utc_millis(unix_ms);
            let whole = rfc3339_utc_seconds(unix_seconds);
            assert_eq!(
                with_fraction,
                format!("{}.{millis:03}Z", &whole[..whole.len() - 1]),
                "{unix_ms}"
            );
            let time = UNIX_EPOCH + Duration::from_millis(unix_ms);
            assert_eq!(iso_from_system_time(time), with_fraction, "{unix_ms}");
        }
    }
}

#[test]
fn the_millisecond_form_parses_back_to_its_instant() {
    for unix_seconds in sweep_seconds().into_iter().step_by(13) {
        let unix_ms = unix_seconds * 1000 + 437;
        let parsed = OffsetDateTime::parse(&rfc3339_utc_millis(unix_ms), &Rfc3339).unwrap();
        assert_eq!(
            parsed.unix_timestamp_nanos(),
            i128::from(unix_ms) * 1_000_000
        );
    }
}

#[test]
fn now_is_the_platform_clock_to_the_second() {
    let before = PlatformClock.now_unix_ms() / 1000;
    let now = now_utc_rfc3339();
    let after = PlatformClock.now_unix_ms() / 1000;
    assert!(
        now == rfc3339_utc_seconds(before) || now == rfc3339_utc_seconds(after),
        "{now} is neither {before} nor {after}"
    );
}
