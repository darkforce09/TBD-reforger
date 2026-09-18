use super::*;

/// The `time_of_day` accept set, pinned against behaviour MEASURED on Postgres 18 rather than
/// assumed — see [`valid_time_of_day`] for why the two columns of this table differ.
///
/// The `false` rows split into two kinds, and both matter:
///
/// * Postgres would REJECT them (`"   "`, `"not-a-time"`, `"25:00"`, `"06:60"`, `"+6:00"`, `""`).
///   Each of those is a 500 without this predicate; each is a 400 with it. `"+6:00"` is the one
///   Rust's `u32::from_str` would let through on its own — it takes a leading `+`.
/// * Postgres would ACCEPT them (`"24:00"`, `"0600"`, `"4:05 PM"`, `"allballs"`,
///   `"06:00:00.5"`, `"06:00:60"`). Those are refused on purpose: they store fine and are then
///   unreadable to `eden_chrome::hhmm_to_minutes`, which parks the author's scrubber back at the
///   06:00 default without saying anything. Letting one in through this door recreates that bug.
#[test]
fn time_of_day_accepts_the_clocks_the_platform_can_round_trip() {
    for (input, accepted) in [
        // What every producer on this path actually emits.
        ("06:00", true),
        ("06:00:00", true),
        ("6:00", true),
        ("23:59:59", true),
        ("00:00", true),
        ("21:45:00", true),
        // Postgres rejects these — each would be a 500.
        ("", false),
        ("   ", false),
        ("\t", false),
        ("not-a-time", false),
        ("25:00", false),
        ("06:60", false),
        ("+6:00", false),
        (" 6:00", false),
        ("06:00:", false),
        ("06:00:00:00", false),
        // Postgres ACCEPTS these; the editor cannot read them back.
        ("24:00", false),
        ("0600", false),
        ("4:05 PM", false),
        ("allballs", false),
        ("06:00:00.5", false),
        ("06:00:60", false),
    ] {
        assert_eq!(
            valid_time_of_day(input).is_some(),
            accepted,
            "time_of_day {input:?}"
        );
    }
}

/// The value is stored as the author wrote it. This layer REJECTS; it does not repair — a
/// one-sided normalisation of a column two sites write is how the two sites come apart.
#[test]
fn an_accepted_time_of_day_is_returned_verbatim() {
    assert_eq!(valid_time_of_day("6:00"), Some("6:00"));
    assert_eq!(valid_time_of_day("06:00:00"), Some("06:00:00"));
}

/// Residual pin — the thumbnail write guard and its message must stay.
#[test]
fn thumbnail_url_write_guard_still_present() {
    const SRC: &str = include_str!("../mission_fields.rs");
    assert!(SRC.contains("fn validated_thumbnail_url("));
    assert!(SRC.contains("thumbnail_url must be an absolute http:// or https:// URL"));
    assert!(validated_thumbnail_url("javascript:alert(1)").is_err());
    assert!(validated_thumbnail_url("https://cdn.example/t.jpg").is_ok());
    assert_eq!(validated_thumbnail_url("").unwrap(), "");
}

/// CREATE + PATCH title: trim and non-empty. Empty / whitespace-only reject; padded non-empty
/// stores trimmed.
///
/// RED: change the helper to bare `is_empty()` — `"   "` would pass and reopen the CREATE hole.
#[test]
fn mission_title_rejects_blank_and_whitespace_only() {
    assert!(validated_mission_title("").is_err());
    assert!(validated_mission_title("   ").is_err());
    assert!(validated_mission_title("\t\n").is_err());
    assert_eq!(
        validated_mission_title("Op Red Dawn").unwrap(),
        "Op Red Dawn"
    );
    assert_eq!(validated_mission_title("  padded  ").unwrap(), "padded");
}

/// Blank weather is not Clear. PATCH `{"weather":""}` must not rewrite dense_fog→clear.
///
/// Aligns with compile/flatten: `""` is not-authored, not a Clear preset.
///
/// RED: put `"" | "clear"` back in [`valid_weather`] — this assert fails (Some(Clear)).
#[test]
fn blank_weather_is_not_clear() {
    assert_eq!(
        valid_weather(""),
        None,
        "empty string must not coerce to WeatherType::Clear (the silent dense_fog→clear rewrite)"
    );
    assert_eq!(valid_weather("clear"), Some(WeatherType::Clear));
    assert_eq!(valid_weather("overcast"), Some(WeatherType::Overcast));
    assert_eq!(valid_weather("heavy_rain"), Some(WeatherType::HeavyRain));
    assert_eq!(valid_weather("dense_fog"), Some(WeatherType::DenseFog));
    assert_eq!(valid_weather("blizzard"), None);
    assert_eq!(valid_weather("   "), None);
}
