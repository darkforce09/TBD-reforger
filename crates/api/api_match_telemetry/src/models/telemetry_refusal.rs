//! The refusals of the match-telemetry ingest routes, each carrying a machine-readable
//! `details.code`.
//!
//! **Role:** builds the 400 and 409 answers a game runtime classifies by code.
//! **Position:** used by the wire decoders in this folder and by the ingest services.
//! **Signals & state:** none; pure constructors.
//! **Invariants:** every refusal a runtime must act on is a 400 or 409 with `details.code` (never a
//! 404, which the mod treats as permanent for any path); `index` names the offending entry of the
//! submitted array, `field` the offending key.
//! @contract match-telemetry.schema.json#/definitions/TelemetryRefusal

use axum::http::StatusCode;
use serde_json::{Value, json};

use crate::Error;
use api_foundation::error_handling::api_error::ApiError;

/// A 400 naming the refusal `code`, and the array `index` and `field` when there is one.
pub fn invalid_entry(
    code: &'static str,
    message: impl Into<String>,
    index: Option<usize>,
    field: Option<&str>,
) -> ApiError {
    let mut details = json!({ "code": code });
    if let Some(index) = index {
        details["index"] = json!(index);
    }
    if let Some(field) = field {
        details["field"] = json!(field);
    }
    ApiError::with_details(StatusCode::BAD_REQUEST, message, details)
}

/// A 409 carrying `code` plus the facts in `details` (an object).
pub fn telemetry_conflict(
    code: &'static str,
    message: impl Into<String>,
    details: Value,
) -> ApiError {
    let mut details = details;
    details["code"] = json!(code);
    ApiError::with_details(StatusCode::CONFLICT, message, details)
}

/// The key a serde decode error names (`unknown field `x``, `missing field `x``), if any.
pub fn field_named_by(error: &serde_json::Error) -> Option<String> {
    let text = error.to_string();
    let start = text.find('`')? + 1;
    let end = start + text[start..].find('`')?;
    Some(text[start..end].to_owned())
}

/// Unknown or malformed keys of a top-level request object: every key must be in `allowed` and
/// every key of `required` present; the first violation is named ([`Error::offending_key`]).
pub fn check_object_keys(value: &Value, allowed: &[&str], required: &[&str]) -> crate::Result<()> {
    let Some(object) = value.as_object() else {
        return Err(Error::BodyNotAnObject);
    };
    if let Some(unknown) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(Error::UnknownField(unknown.clone()));
    }
    if let Some(missing) = required.iter().find(|key| !object.contains_key(**key)) {
        return Err(Error::MissingField((*missing).to_owned()));
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/telemetry_refusal.rs"]
mod tests;
