//! The shape failures' messages and keys: the exact text and `details.field` a game runtime reads
//! in the refusal its decoder builds from them.

use super::Error;
use api_foundation::error_handling::api_error::ApiError;
use axum::http::StatusCode;

#[test]
fn each_failure_spells_the_refusal_message_and_names_its_key() {
    let cases = [
        (
            Error::SourceMatchIdLength,
            "source_match_id must contain 1 to 128 bytes",
            "source_match_id",
        ),
        (Error::BodyNotAnObject, "the body must be a JSON object", ""),
        (
            Error::UnknownField("extra".to_owned()),
            "unknown field `extra`",
            "extra",
        ),
        (
            Error::MissingField("players".to_owned()),
            "missing field `players`",
            "players",
        ),
    ];
    for (failure, message, key) in cases {
        assert_eq!(failure.to_string(), message);
        assert_eq!(failure.offending_key(), key);
    }
}

#[test]
fn a_failure_without_a_refusal_code_is_a_plain_400() {
    let error = ApiError::from(Error::MissingField("revision".to_owned()));
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "missing field `revision`");
}
