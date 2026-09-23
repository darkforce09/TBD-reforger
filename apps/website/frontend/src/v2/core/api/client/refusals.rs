//! A refused request kept whole: the status, the backend's sentence, and the structured reason.
//!
//! **Role:** carries the `details` object a refusal body names its reason in, for the routes whose
//! callers branch on that reason — a registration refused by an access policy is told apart from
//! one refused because the operation is full, and a stale access revision from any other conflict.
//! **Position:** the failure half of the refusal-keeping verbs in the request module. A page reads
//! the reason through [`ApiRefusal::code`] and [`ApiRefusal::detail`], and the sentence through
//! [`ApiRefusal::message_or`].
//! **Signals & state:** none. Every function is pure over its arguments.
//! **Invariants:** the status keeps the meanings [`ApiErr`] gives it — `0` for a request that never
//! reached the backend or an answer that could not be read, `401` for a session that is over. Only
//! an object `details` is kept as the reason; an array of findings is folded into the message
//! instead, exactly as [`error_body_message`] folds it for every other request, so both kinds of
//! refusal still read as prose.

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use super::errors::{api_error_message, error_body_message, ApiErr};

/// A request the backend did not carry out, with the reason its error body names.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct ApiRefusal {
    /// The HTTP status; `0` when the request never reached the backend or the answer was unreadable.
    pub status: u16,
    /// The backend's `error` sentence, with any findings folded in as extra lines.
    pub message: Option<String>,
    /// The `details` object the route attached, when it sent one.
    pub details: Option<Map<String, Value>>,
}

#[allow(dead_code)]
impl ApiRefusal {
    /// A request that produced nothing readable: never sent, never answered, or answered with a
    /// body that is not the shape asked for.
    pub fn unreadable() -> Self {
        Self {
            status: 0,
            message: None,
            details: None,
        }
    }

    /// Read a non-2xx answer: its status, and its body when that parsed as JSON.
    pub fn from_error_body(status: u16, body: Option<&Value>) -> Self {
        Self {
            status,
            message: body.and_then(error_body_message),
            details: body
                .and_then(|body| body.get("details"))
                .and_then(Value::as_object)
                .cloned(),
        }
    }

    /// `details.code`, the machine-readable reason, when the route named one.
    pub fn code(&self) -> Option<&str> {
        self.detail("code")
    }

    /// One text field of `details`, such as `policy_source` or `opens_at`.
    pub fn detail(&self, key: &str) -> Option<&str> {
        self.details.as_ref()?.get(key)?.as_str()
    }

    /// The backend's sentence with its first letter capitalised, or `fallback` when it sent none.
    pub fn message_or(&self, fallback: &str) -> String {
        api_error_message(&(self.status, self.message.clone()), fallback)
    }

    /// True only for a terminal `401`: the session is over, whatever the route.
    pub fn is_session_expired(&self) -> bool {
        self.status == 401
    }
}

/// A failure that carried no readable body — the session, the network, or an unreadable answer.
impl From<ApiErr> for ApiRefusal {
    fn from((status, message): ApiErr) -> Self {
        Self {
            status,
            message,
            details: None,
        }
    }
}

/// Decode an answer the backend gave — its status, and its body when that parsed as JSON.
///
/// A 2xx body becomes `T`; one that does not deserialise into `T` is the unreadable answer the
/// other verbs report as status `0`. Any other status becomes the refusal its body describes.
#[allow(dead_code)]
pub fn decode_answer<T: DeserializeOwned>(
    status: u16,
    body: Option<Value>,
) -> Result<T, ApiRefusal> {
    if (200..300).contains(&status) {
        body.and_then(|body| serde_json::from_value(body).ok())
            .ok_or_else(ApiRefusal::unreadable)
    } else {
        Err(ApiRefusal::from_error_body(status, body.as_ref()))
    }
}

#[cfg(test)]
#[path = "tests/refusals.rs"]
mod tests;
