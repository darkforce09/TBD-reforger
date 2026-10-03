//! Unit tests for the event creation rules: each refusal names the field that broke its rule, and
//! an accepted request stores its values as the rules normalise them.

use axum::http::StatusCode;
use chrono::{DateTime, TimeZone, Timelike, Utc};

use super::{EventCreation, EventCreationRequest, check_name_override, validated_banner_image_url};
use crate::models::EventStatus;

fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2027, 3, 3, 18, 0, 0).unwrap()
}

fn request() -> EventCreationRequest {
    EventCreationRequest {
        start_time: Some(start()),
        ..EventCreationRequest::default()
    }
}

fn refusal(request: EventCreationRequest) -> String {
    let error = EventCreation::new(request).expect_err("the request breaks a rule");
    assert_eq!(error.status, StatusCode::BAD_REQUEST, "{}", error.message);
    error.message
}

#[test]
fn event_creation_requires_a_start_time() {
    let missing = EventCreationRequest {
        start_time: None,
        max_slots: 999,
        ..EventCreationRequest::default()
    };
    assert_eq!(refusal(missing), "start_time is required");
}

/// A place cap outside 0–256 is refused under its own name, not as a missing start time.
#[test]
fn event_creation_names_a_place_cap_out_of_range() {
    for max_slots in [-1, 257, i64::MAX] {
        let message = refusal(EventCreationRequest {
            max_slots,
            ..request()
        });
        assert_eq!(
            message, "max_slots must be between 0 and 256",
            "{max_slots}"
        );
    }
    for max_slots in [0, 128, 256] {
        let creation = EventCreation::new(EventCreationRequest {
            max_slots,
            ..request()
        })
        .expect("a place cap inside 0-256");
        assert_eq!(creation.max_slots(), max_slots);
    }
}

#[test]
fn event_creation_accepts_only_pre_start_statuses() {
    for (spelled, status) in [
        ("", EventStatus::Scheduled),
        ("scheduled", EventStatus::Scheduled),
        ("open", EventStatus::Open),
        ("locked", EventStatus::Locked),
    ] {
        let creation = EventCreation::new(EventCreationRequest {
            status: spelled.to_owned(),
            ..request()
        })
        .expect("a pre-start status");
        assert_eq!(creation.status(), status, "{spelled:?}");
    }
    for spelled in ["live", "completed", "cancelled"] {
        assert_eq!(
            refusal(EventCreationRequest {
                status: spelled.to_owned(),
                ..request()
            }),
            "an event may only be created as scheduled, open or locked",
            "{spelled}"
        );
    }
    assert_eq!(
        refusal(EventCreationRequest {
            status: "Open".to_owned(),
            ..request()
        }),
        "invalid status"
    );
}

#[test]
fn event_creation_stores_the_start_time_to_the_microsecond() {
    let precise = start().with_nanosecond(123_456_789).unwrap();
    let creation = EventCreation::new(EventCreationRequest {
        start_time: Some(precise),
        ..request()
    })
    .expect("a valid start time");
    assert_eq!(creation.start_time().nanosecond(), 123_456_000);
}

#[test]
fn event_creation_refuses_a_blank_name_and_a_relative_banner() {
    assert!(
        refusal(EventCreationRequest {
            name_override: "   ".to_owned(),
            ..request()
        })
        .starts_with("name_override must not be blank")
    );
    assert!(check_name_override("").is_ok());
    assert!(check_name_override("Operation Dawn").is_ok());
    assert_eq!(
        refusal(EventCreationRequest {
            banner_image_url: "/banner.png".to_owned(),
            ..request()
        }),
        "banner_image_url must be an absolute http:// or https:// URL"
    );
    assert_eq!(validated_banner_image_url("").unwrap(), "");
    assert_eq!(
        validated_banner_image_url("  https://example.invalid/banner.png ").unwrap(),
        "https://example.invalid/banner.png"
    );
}
