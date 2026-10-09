//! A refused vehicle write: the backend's field sentence, and an oversized request named whether
//! the backend or a proxy refused it.

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
