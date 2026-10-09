//! Unit coverage for [`required_trimmed_text`].
//!
//! **Role:** pins the trim, the blank refusal and its message.
//! **Position:** sibling test module of `api_foundation::http::required_text_field`.
//! **Signals & state:** none.
//! **Invariants:** surrounding whitespace is removed and inner whitespace kept; an empty or
//! whitespace-only value answers `400 <field> is required`.

use axum::http::StatusCode;

use super::*;

#[test]
fn a_value_is_returned_trimmed_with_inner_whitespace_kept() {
    assert_eq!(
        required_trimmed_text("  Alpha Pack  ", "name").expect("a non-blank value"),
        "Alpha Pack"
    );
    assert_eq!(
        required_trimmed_text("1.2.3", "version").expect("a non-blank value"),
        "1.2.3"
    );
}

#[test]
fn an_empty_value_answers_400_naming_the_field() {
    let refusal = required_trimmed_text("", "name").expect_err("an empty value is refused");
    assert_eq!(refusal.status, StatusCode::BAD_REQUEST);
    assert_eq!(refusal.message, "name is required");
}

#[test]
fn a_whitespace_only_value_answers_400_naming_the_field() {
    let refusal = required_trimmed_text(" \t\n ", "version").expect_err("a blank value is refused");
    assert_eq!(refusal.status, StatusCode::BAD_REQUEST);
    assert_eq!(refusal.message, "version is required");
}
