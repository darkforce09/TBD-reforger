use serde_json::json;

use super::*;

#[test]
fn a_reply_within_the_bound_is_kept_whole() {
    let exactly_the_bound = "x".repeat(CONSOLE_RESPONSE_MAX_BYTES);
    for reply in [
        "",
        "Players on server:\n0 ; 3d8f0e52-6c5a-4e0b-9a55-0c2b1d7e4f10 ; Jérôme",
        exactly_the_bound.as_str(),
    ] {
        assert_eq!(
            ConsoleResponseCapture::of(reply),
            ConsoleResponseCapture {
                response: reply.to_owned(),
                response_truncated: false,
            }
        );
    }
}

#[test]
fn a_longer_reply_is_cut_at_the_bound() {
    let reply = "x".repeat(CONSOLE_RESPONSE_MAX_BYTES + 1);
    let capture = ConsoleResponseCapture::of(&reply);
    assert_eq!(capture.response, reply[..CONSOLE_RESPONSE_MAX_BYTES]);
    assert!(capture.response_truncated);
}

#[test]
fn a_character_across_the_bound_is_left_out_whole() {
    // A three-byte character starting two bytes before the bound.
    let reply = format!("{}€ and more", "x".repeat(CONSOLE_RESPONSE_MAX_BYTES - 2));
    let capture = ConsoleResponseCapture::of(&reply);
    assert_eq!(capture.response, reply[..CONSOLE_RESPONSE_MAX_BYTES - 2]);
    assert!(capture.response_truncated);
    // A character that ends exactly at the bound is kept.
    let reply = format!("{}€ and more", "x".repeat(CONSOLE_RESPONSE_MAX_BYTES - 3));
    let capture = ConsoleResponseCapture::of(&reply);
    assert_eq!(capture.response, reply[..CONSOLE_RESPONSE_MAX_BYTES]);
    assert!(capture.response.ends_with('€'));
    assert!(capture.response_truncated);
}

#[test]
fn the_outcome_holds_the_response_and_whether_it_was_cut() {
    assert_eq!(
        Value::Object(ConsoleResponseCapture::of("executed #restart").into_outcome()),
        json!({"response": "executed #restart", "response_truncated": false})
    );
    let outcome = ConsoleResponseCapture::of(&"y".repeat(5_000)).into_outcome();
    assert_eq!(
        Value::Object(outcome),
        json!({"response": "y".repeat(CONSOLE_RESPONSE_MAX_BYTES), "response_truncated": true})
    );
}
