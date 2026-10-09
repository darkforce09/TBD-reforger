//! The required text field rule a JSON write body applies to a field that must not be blank.
//!
//! **Role:** trims a required text field and refuses it when nothing is left.
//! **Position:** `api_foundation::http`; the modpack writes of `api_community_content` (name and
//! version) and the server registry writes of `api_server_infrastructure` (name) call
//! [`required_trimmed_text`] on their request bodies.
//! **Signals & state:** none; a pure function.
//! **Invariants:** the value returned is the value stored, trimmed of leading and trailing
//! whitespace; an empty or whitespace-only value answers `400` with `<field> is required`; no
//! length cap (the JSON body limit is the boundary).

use crate::error_handling::api_error::ApiError;

/// The trimmed `raw` value of the required text field `field`, or a `400` failure whose message
/// is `<field> is required` when the trimmed value is empty.
///
/// # Errors
///
/// [`ApiError::bad_request`] when `raw` is empty or whitespace only.
pub fn required_trimmed_text(raw: &str, field: &str) -> Result<String, ApiError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(ApiError::bad_request(format!("{field} is required")));
    }
    Ok(value.to_string())
}

#[cfg(test)]
#[path = "tests/required_text_field.rs"]
mod tests;
