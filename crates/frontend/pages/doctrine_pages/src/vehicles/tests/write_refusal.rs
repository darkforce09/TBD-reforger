//! The wording of a refused vehicle write, per status the backend or a proxy can answer.

use super::refusal_sentence;
use frontend_transport::Error;
use serde_json::json;

const FALLBACK: &str = "The vehicle could not be saved.";

fn refused(status: u16, body: serde_json::Value) -> Error {
    Error::from_error_body(status, Some(&body))
}

#[test]
fn a_400_shows_the_backends_field_sentence() {
    let refusal = refused(400, json!({"error": "name must be at most 120 characters"}));
    assert_eq!(
        refusal_sentence(&refusal, FALLBACK),
        "Name must be at most 120 characters"
    );
    let image = refused(
        400,
        json!({"error": "profile_image_url must be an https:// URL or a site path such as /uploads/…"}),
    );
    assert!(refusal_sentence(&image, FALLBACK).starts_with("Profile_image_url must be"));
}

#[test]
fn a_400_without_a_sentence_shows_the_fallback() {
    let refusal = Error::from_error_body(400, None);
    assert_eq!(refusal_sentence(&refusal, FALLBACK), FALLBACK);
}

#[test]
fn an_oversized_request_is_named_whether_the_backend_or_a_proxy_refused_it() {
    let backend = refused(
        413,
        json!({"error": "request body too large", "details": {"code": "request_too_large"}}),
    );
    let proxy = Error::from_error_body(413, None);
    for refusal in [backend, proxy] {
        assert_eq!(
            refusal_sentence(&refusal, FALLBACK),
            "The vehicle is too large to send. Shorten its fields and try again."
        );
    }
}

#[test]
fn a_403_says_the_database_is_for_administrators() {
    let refusal = refused(403, json!({"error": "forbidden"}));
    assert_eq!(
        refusal_sentence(&refusal, FALLBACK),
        "Only administrators can change the vehicle database."
    );
}

#[test]
fn a_missing_vehicle_an_ended_session_and_an_unreadable_answer_each_have_a_sentence() {
    let missing = refused(404, json!({"error": "vehicle not found"}));
    assert!(refusal_sentence(&missing, FALLBACK).starts_with("This vehicle is no longer"));
    let expired = Error::from_status(401, None);
    assert!(refusal_sentence(&expired, FALLBACK).starts_with("Your session has ended"));
    let unreadable = Error::Transport;
    assert!(
        refusal_sentence(&unreadable, FALLBACK).starts_with("The platform could not be reached")
    );
}

#[test]
fn any_other_status_shows_the_backends_sentence_or_the_fallback() {
    let with_sentence = refused(500, json!({"error": "database unavailable"}));
    assert_eq!(
        refusal_sentence(&with_sentence, FALLBACK),
        "Database unavailable"
    );
    let without = Error::from_error_body(502, None);
    assert_eq!(refusal_sentence(&without, FALLBACK), FALLBACK);
}
