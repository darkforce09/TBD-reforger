//! The JSON wire contract, pinned at the encoder: enum values are snake_case, optional fields
//! are absent (not `null`) when empty, nullable non-optional fields serialize as `null`,
//! timestamps are RFC 3339 UTC with trailing fractional zeros trimmed, and dates are midnight
//! UTC. The SPA's DTO golden tests and the Enfusion mod parse exactly these spellings.

use chrono::{TimeZone, Utc};
use serde_json::json;
use website_api::core::wire_format::rfc3339_utc;
use website_api::identity_and_access::models::user_account::{User, UserRole};
use website_api::missions::models::mission::{GameMode, MissionStatus, WeatherType};
use website_api::operations::models::event::RegistrationState;

#[test]
fn enum_values_are_snake_case() {
    assert_eq!(
        serde_json::to_value(UserRole::MissionMaker).unwrap(),
        json!("mission_maker")
    );
    assert_eq!(
        serde_json::to_value(UserRole::Enlisted).unwrap(),
        json!("enlisted")
    );
    assert_eq!(
        serde_json::to_value(GameMode::PveCoop).unwrap(),
        json!("pve_coop")
    );
    assert_eq!(
        serde_json::to_value(WeatherType::HeavyRain).unwrap(),
        json!("heavy_rain")
    );
    assert_eq!(
        serde_json::to_value(RegistrationState::NoShow).unwrap(),
        json!("no_show")
    );
    assert_eq!(
        serde_json::to_value(MissionStatus::PendingApproval).unwrap(),
        json!("pending_approval")
    );
}

#[test]
fn timestamps_trim_trailing_fractional_zeros() {
    let whole = Utc.with_ymd_and_hms(2026, 7, 6, 12, 0, 0).unwrap();
    assert_eq!(rfc3339_utc::format(&whole), "2026-07-06T12:00:00Z");

    // 500 ms → ".5" (trailing zeros trimmed), not ".500".
    let half = whole + chrono::Duration::milliseconds(500);
    assert_eq!(rfc3339_utc::format(&half), "2026-07-06T12:00:00.5Z");

    // Full nanosecond precision, no padding artifacts.
    let nanos = whole + chrono::Duration::nanoseconds(123_456_789);
    assert_eq!(
        rfc3339_utc::format(&nanos),
        "2026-07-06T12:00:00.123456789Z"
    );
}

#[test]
fn user_optional_fields_are_absent_and_nullable_fields_are_null() {
    let dt = Utc.with_ymd_and_hms(2026, 7, 6, 12, 0, 0).unwrap();
    let u = User {
        discord_id: "1".into(),
        username: "Dave".into(),
        discord_handle: String::new(),
        avatar_url: String::new(),
        arma_id: None,
        arma_character: String::new(),
        role: UserRole::Admin,
        is_banned: false,
        ban_reason: String::new(),
        banned_by: None,
        banned_at: None,
        total_deployments: 42,
        attendance_rate: 94.0,
        last_login_at: None,
        created_at: dt,
        updated_at: dt,
    };
    let v = serde_json::to_value(&u).unwrap();
    let obj = v.as_object().unwrap();

    // arma_id is nullable, not optional → present as null.
    assert_eq!(obj.get("arma_id"), Some(&serde_json::Value::Null));
    // Non-omitempty empty strings still serialize.
    assert!(obj.contains_key("discord_handle"));
    assert!(obj.contains_key("avatar_url"));
    // omitempty fields are ABSENT (not null) when empty/none.
    for absent in ["ban_reason", "banned_by", "banned_at", "last_login_at"] {
        assert!(
            !obj.contains_key(absent),
            "{absent} must be omitted when empty"
        );
    }
    // enum snake_case + timestamp + numeric.
    assert_eq!(obj["role"], json!("admin"));
    assert_eq!(obj["created_at"], json!("2026-07-06T12:00:00Z"));
    assert_eq!(obj["total_deployments"], json!(42));

    // No camelCase leaked into a DB model (the census invariant: 0 camelCase tags).
    assert!(
        obj.keys()
            .all(|k| !k.chars().any(|c| c.is_ascii_uppercase()))
    );
}
