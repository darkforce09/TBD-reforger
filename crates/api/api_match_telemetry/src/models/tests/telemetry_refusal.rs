use axum::http::StatusCode;
use serde_json::json;

use super::{check_object_keys, field_named_by, invalid_entry, telemetry_conflict};
use crate::Error;

#[test]
fn an_invalid_entry_is_a_400_naming_code_index_and_field() {
    let error = invalid_entry("INVALID_EVENT", "bad", Some(3), Some("sequence"));
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        error.details,
        Some(json!({"code": "INVALID_EVENT", "index": 3, "field": "sequence"}))
    );
}

#[test]
fn a_conflict_is_a_409_carrying_its_code_and_facts() {
    let error = telemetry_conflict("STALE_REVISION", "old", json!({"revision": 4}));
    assert_eq!(error.status, StatusCode::CONFLICT);
    assert_eq!(
        error.details,
        Some(json!({"revision": 4, "code": "STALE_REVISION"}))
    );
}

#[test]
fn serde_errors_name_their_field() {
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct Probe {
        kept: i64,
    }
    let unknown = serde_json::from_value::<Probe>(json!({"kept": 1, "kills": 2})).unwrap_err();
    assert_eq!(field_named_by(&unknown).as_deref(), Some("kills"));
    let missing = serde_json::from_value::<Probe>(json!({})).unwrap_err();
    assert_eq!(field_named_by(&missing).as_deref(), Some("kept"));
}

#[test]
fn object_keys_are_checked_for_unknown_then_missing() {
    let allowed = ["a", "b"];
    assert!(check_object_keys(&json!({"a": 1}), &allowed, &["a"]).is_ok());
    assert_eq!(
        check_object_keys(&json!({"a": 1, "c": 2}), &allowed, &["a"]).unwrap_err(),
        Error::UnknownField("c".to_owned())
    );
    assert_eq!(
        check_object_keys(&json!({"b": 1}), &allowed, &["a"]).unwrap_err(),
        Error::MissingField("a".to_owned())
    );
    assert_eq!(
        check_object_keys(&json!([1]), &allowed, &[]).unwrap_err(),
        Error::BodyNotAnObject
    );
}
