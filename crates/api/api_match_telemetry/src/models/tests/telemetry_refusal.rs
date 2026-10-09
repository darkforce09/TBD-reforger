use axum::http::StatusCode;
use serde_json::json;

use super::invalid_entry;

#[test]
fn an_invalid_entry_is_a_400_naming_code_index_and_field() {
    let error = invalid_entry("INVALID_EVENT", "bad", Some(3), Some("sequence"));
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        error.details,
        Some(json!({"code": "INVALID_EVENT", "index": 3, "field": "sequence"}))
    );
}
