//! Shared handler error type. Serializes to Go's error envelope
//! `{"error": msg}` (plus optional `"details"`), with the mapped HTTP status.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// A handler failure carrying an HTTP status, a client message, and optional
/// structured `details` (schema-validation messages, mortar partial solution, …).
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            details: None,
        }
    }

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

    pub fn bad_request(m: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, m)
    }
    pub fn unauthorized(m: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, m)
    }
    pub fn forbidden(m: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, m)
    }
    pub fn not_found(m: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, m)
    }
    pub fn conflict(m: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, m)
    }
    pub fn internal(m: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, m)
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
