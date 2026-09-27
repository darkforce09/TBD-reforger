//! The shared handler failure and the JSON error envelope it renders into.
//!
//! **Role:** [`ApiError`] carries the HTTP status, the client message and optional structured
//! details of every handler and service failure, and renders them as `{"error": msg}` plus an
//! optional `"details"` value.
//!
//! **Position:** `core`; every domain's handlers and services return it, and axum turns it into
//! the response through [`IntoResponse`]. [`ApiError::from_json_rejection`] maps axum's JSON body
//! rejections into it, [`ApiError::from_query_rejection`] its query string rejections, and a
//! `sqlx::Error` converts into it.
//!
//! **Signals & state:** none; plain values.
//!
//! **Invariants:** `error` is always a string and `details` is present only when set, the shape
//! the single-page app parses; a database error reaches the client as `internal error`, never as
//! its text; a JSON body over the request limit answers 413 with `details.code =
//! "request_too_large"`, a missing or wrong JSON content type 415, and every other unreadable JSON
//! body 400 with the rejection's reason as its message; a query string that does not decode
//! answers 400 with a message naming the query and the rejection's reason.

use axum::Json;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// The `details.code` of a JSON body refused for exceeding the request body limit.
const REQUEST_TOO_LARGE_CODE: &str = "request_too_large";

/// A handler failure carrying an HTTP status, a client message, and optional
/// structured `details` (schema-validation messages, mortar partial solution, …).
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    /// A failure with any status and no details.
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            details: None,
        }
    }

    /// A failure with any status and a structured `details` value.
    pub fn with_details(
        status: StatusCode,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            status,
            message: message.into(),
            details: Some(details),
        }
    }

    /// A `400 Bad Request` failure.
    pub fn bad_request(m: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, m)
    }
    /// A `401 Unauthorized` failure.
    pub fn unauthorized(m: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, m)
    }
    /// A `403 Forbidden` failure.
    pub fn forbidden(m: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, m)
    }
    /// A `404 Not Found` failure.
    pub fn not_found(m: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, m)
    }
    /// A `409 Conflict` failure.
    pub fn conflict(m: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, m)
    }
    /// A `500 Internal Server Error` failure.
    pub fn internal(m: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, m)
    }

    /// The failure a handler answers when axum's [`axum::Json`] extractor refuses the body.
    ///
    /// - A body over the request body limit answers `413 Payload Too Large` with
    ///   `details.code = "request_too_large"`.
    /// - A missing or non-JSON `Content-Type` answers `415 Unsupported Media Type`.
    /// - Every other refusal (malformed JSON, a body that does not decode into the target type,
    ///   a body that cannot be read) answers `400 Bad Request` with the rejection's text as the
    ///   message, so the caller learns which field or byte failed.
    ///
    /// The classification reads the rejection's own status, so it holds for every rejection
    /// variant axum defines.
    pub fn from_json_rejection(rejection: JsonRejection) -> Self {
        match rejection.status() {
            StatusCode::PAYLOAD_TOO_LARGE => Self::with_details(
                StatusCode::PAYLOAD_TOO_LARGE,
                "request body is too large",
                json!({ "code": REQUEST_TOO_LARGE_CODE }),
            ),
            StatusCode::UNSUPPORTED_MEDIA_TYPE => {
                Self::new(StatusCode::UNSUPPORTED_MEDIA_TYPE, rejection.body_text())
            }
            _ => Self::bad_request(rejection.body_text()),
        }
    }

    /// The failure a handler answers when axum's [`axum::extract::Query`] extractor refuses the
    /// query string: `400 Bad Request` in the `{error, details?}` envelope, never axum's
    /// plain-text rejection body.
    ///
    /// The message reads `invalid <query_name>: <reason>`, where the reason is the rejection's
    /// text and names the parameter that failed, e.g. `invalid announcement page query: Failed to
    /// deserialize query string: limit: invalid digit found in string`.
    pub fn from_query_rejection(rejection: QueryRejection, query_name: &str) -> Self {
        Self::bad_request(format!("invalid {query_name}: {}", rejection.body_text()))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut body = json!({ "error": self.message });
        if let Some(details) = self.details {
            body.as_object_mut()
                .expect("object")
                .insert("details".into(), details);
        }
        (self.status, Json(body)).into_response()
    }
}

/// Any unhandled DB error maps to a logged 500 (handlers map the cases that need a
/// specific status — 409 unique violation, etc. — explicitly).
impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!(error = %e, "database error");
        Self::internal("internal error")
    }
}

#[cfg(test)]
#[path = "tests/api_error.rs"]
mod tests;
