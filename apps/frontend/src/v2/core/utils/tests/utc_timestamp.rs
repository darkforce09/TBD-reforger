//! UTC instants: read in every spelling the wire uses, compared by value, and written back.

use super::*;

/// The wire's own spellings — trimmed fraction, full micros, whole seconds — all read.
#[test]
fn the_wire_spellings_parse_and_write_back_unchanged() {
    for wire in [
        "2026-07-15T14:05:44.629713Z",
        "2026-07-15T14:20:00Z",
        "2026-08-01T19:00:00.5Z",
        "2024-02-29T23:59:59.123456789Z",
    ] {
        let instant = UtcTimestamp::parse(wire).unwrap_or_else(|| panic!("{wire} must parse"));
        assert_eq!(instant.to_rfc3339(), wire);
    }
    assert_eq!(
        UtcTimestamp::parse("2026-07-15T14:20:00+00:00")
            .unwrap()
            .to_rfc3339(),
        "2026-07-15T14:20:00Z"
    );
}

/// A non-UTC offset, an impossible date or a malformed field is refused rather than guessed at.
#[test]
fn anything_but_a_valid_utc_instant_is_refused() {
    for bad in [
        "2026-07-15T14:20:00+02:00",
        "2026-07-15T14:20:00",
        "2026-02-30T00:00:00Z",
        "2025-02-29T00:00:00Z",
        "2026-13-01T00:00:00Z",
        "2026-07-15T24:00:00Z",
        "2026-07-15T14:20Z",
        "2026-07-15T14:20:00.Z",
        "2026-07-15T14:20:00.1234567890Z",
        "26-07-15T14:20:00Z",
        "",
    ] {
        assert_eq!(UtcTimestamp::parse(bad), None, "{bad:?} must be refused");
    }
}

/// Instants compare by value: a trimmed fraction is later than the whole second before it, which a
/// text comparison of the two spellings gets backwards.
#[test]
fn instants_compare_by_value_not_by_text() {
    let whole = UtcTimestamp::parse("2026-07-15T14:05:44Z").unwrap();
    let later = UtcTimestamp::parse("2026-07-15T14:05:44.629713Z").unwrap();
    assert!(later > whole);
    assert!(
        "2026-07-15T14:05:44.629713Z" < "2026-07-15T14:05:44Z",
        "the text order is wrong"
    );
    let next_day = UtcTimestamp::parse("2026-07-16T00:00:00Z").unwrap();
    assert!(next_day > later);
}

/// The UTC line drops seconds and names the zone; a value that is not a UTC instant is shown as
/// written rather than hidden.
#[test]
fn the_utc_label_names_the_zone_and_keeps_unreadable_text() {
    assert_eq!(
        utc_label("2026-07-15T14:05:44.629713Z"),
        "2026-07-15 14:05 UTC"
    );
    assert_eq!(utc_label("soon"), "soon");
}

/// A date-and-time field's value is read as UTC and written back the same way, so an opening time
/// edited in the field is the instant the field showed.
#[test]
fn the_datetime_field_round_trips_as_utc() {
    let instant = UtcTimestamp::from_datetime_field("2026-08-01T19:30").unwrap();
    assert_eq!(instant.to_rfc3339(), "2026-08-01T19:30:00Z");
    assert_eq!(instant.datetime_field_value(), "2026-08-01T19:30");
    assert_eq!(
        UtcTimestamp::from_datetime_field("2026-08-01T19:30:15")
            .unwrap()
            .to_rfc3339(),
        "2026-08-01T19:30:15Z"
    );
    let shown = UtcTimestamp::parse("2026-07-15T14:05:44.629713Z").unwrap();
    assert_eq!(shown.datetime_field_value(), "2026-07-15T14:05");
    for bad in ["", "2026-08-01", "2026-08-01T25:00", "2026-08-01 19:30"] {
        assert_eq!(UtcTimestamp::from_datetime_field(bad), None, "{bad:?}");
    }
}
