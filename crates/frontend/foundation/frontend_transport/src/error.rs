//! The crate's error: why a request or a stream produced no data.
//!
//! **Role:** the one failure type of every request verb, typed endpoint call, public read and
//! stream setup. It names the ways a request ends without data — the session is over, the API
//! refused (with the reason its body names), the request never reached the API, the answer could
//! not be used, the caller cancelled — so a render site matches on the reason instead of
//! flattening every failure into an empty state.
//! **Position:** built by the request verbs ([`crate::client`]) from the answer's status and error
//! body, carried by [`crate::client::Fetched`] to the pages, and read there through
//! [`Error::status`], [`Error::code`], [`Error::detail`] and [`Error::message_or`].
//! **Signals & state:** none; plain data.
//! **Invariants:** status `0` is [`Error::Transport`] and `401` is [`Error::SessionExpired`]; every
//! other refused status keeps its number in [`Error::Http`]. Only an object `details` is kept as
//! the structured reason; an array of findings is folded into the message instead. Each variant's
//! `Display` text is the sentence the interface shows for it. There is no "empty" variant.

use serde_json::{Map, Value};

use crate::client::errors::error_body_message;

/// A result whose failure is the crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Why a request produced no data.
///
/// The point of this enum is that "empty" is not one of its variants. A bare `(status, message)`
/// pair invites `.ok` at the call site, which flattens every failure into `None` and every `None`
/// into an empty render — so a dead session shows up as a blank page, and the interface reports
/// "nothing here" over data it was never allowed to read. Naming the states is what carries the
/// refusal all the way to the render site.
///
/// A `401` reaching here always means the session rather than the route: the backend answers `401`
/// only from the auth middleware, for a missing or unparseable bearer token, and insufficient role
/// is `403`. By the time a failure escapes the request verbs' refresh policy
/// ([`crate::client::refresh`]) a `401` has already survived a single-flight refresh and a retry
/// with the rotated token.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The session is over: refresh and retry both failed. Surface "session expired — log in
    /// again", never an empty state.
    #[error("the session has expired")]
    SessionExpired {
        /// The backend's error string, when it sent one.
        message: Option<String>,
    },
    /// The backend answered and refused for a reason that is not authentication. The caller maps
    /// the status or the reason code; the message is the human-readable cause.
    #[error("the API refused the request with status {status}")]
    Http {
        /// The HTTP status the backend answered with.
        status: u16,
        /// The backend's `error` sentence, with any findings folded in as extra lines.
        message: Option<String>,
        /// The `details` object the route attached naming its reason, when it sent one.
        details: Option<Map<String, Value>>,
    },
    /// The request never reached the backend — network down, blocked by CORS, or a body that would
    /// not deserialise. Distinct from [`Self::SessionExpired`] because "you are logged out" is a
    /// lie when the truth is "the network dropped".
    #[error("the request never reached the API")]
    Transport,
    /// The request produced no usable answer, for a reason worded where it failed: a network
    /// error's own text, a body that would not decode, an answer missing what it must carry, or a
    /// stream step that could not be taken.
    #[error("{reason}")]
    Failed {
        /// The sentence naming what failed.
        reason: String,
    },
    /// The caller aborted the request before it settled.
    #[error("Request cancelled")]
    Cancelled,
    /// A public read was asked for a path that is not a site path (`/…`, never `//host`).
    #[error("Invalid public API path")]
    NotASitePath,
    /// A public read, which carries no credentials, was refused: its status means the route, never
    /// the session.
    #[error("{}", public_refusal_sentence(*.status, .message.as_deref()))]
    PublicRefusal {
        /// The HTTP status the backend answered with.
        status: u16,
        /// The backend's `error` sentence, when it sent one.
        message: Option<String>,
    },
}

/// The sentence of a refused public read: the backend's own, or the status when it sent none.
fn public_refusal_sentence(status: u16, message: Option<&str>) -> String {
    message.map_or_else(|| format!("Request failed ({status})"), str::to_string)
}

impl Error {
    /// Classify a failure that carried no readable body: status `0` is the client's own sentinel
    /// for "never got an answer", `401` is the dead session, and everything else keeps its status.
    pub fn from_status(status: u16, message: Option<String>) -> Self {
        match status {
            0 => Self::Transport,
            401 => Self::SessionExpired { message },
            _ => Self::Http {
                status,
                message,
                details: None,
            },
        }
    }

    /// Read a non-2xx answer: its status, and its body when that parsed as JSON. The `error`
    /// sentence becomes the message (findings folded in), an object `details` becomes the reason.
    pub fn from_error_body(status: u16, body: Option<&Value>) -> Self {
        let details = body
            .and_then(|body| body.get("details"))
            .and_then(Value::as_object)
            .cloned();
        match Self::from_status(status, body.and_then(error_body_message)) {
            Self::Http {
                status, message, ..
            } => Self::Http {
                status,
                message,
                details,
            },
            other => other,
        }
    }

    /// The HTTP status this failure carries: `401` for an expired session, the refused status for
    /// a refusal, and `0` for every failure that got no answer.
    pub fn status(&self) -> u16 {
        match self {
            Self::SessionExpired { .. } => 401,
            Self::Http { status, .. } | Self::PublicRefusal { status, .. } => *status,
            Self::Transport | Self::Failed { .. } | Self::Cancelled | Self::NotASitePath => 0,
        }
    }

    /// The backend's sentence, or the failure site's own reason, when there is one.
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::SessionExpired { message }
            | Self::Http { message, .. }
            | Self::PublicRefusal { message, .. } => message.as_deref(),
            Self::Failed { reason } => Some(reason),
            Self::Transport | Self::Cancelled | Self::NotASitePath => None,
        }
    }

    /// The `details` object a refusal named its reason in, when the route sent one.
    pub fn details(&self) -> Option<&Map<String, Value>> {
        match self {
            Self::Http { details, .. } => details.as_ref(),
            _ => None,
        }
    }

    /// `details.code`, the machine-readable reason, when the route named one.
    pub fn code(&self) -> Option<&str> {
        self.detail("code")
    }

    /// One text field of `details`, such as `policy_source` or `opens_at`.
    pub fn detail(&self, key: &str) -> Option<&str> {
        self.details()?.get(key)?.as_str()
    }

    /// The message with its first letter capitalised, or `fallback` when there is none.
    pub fn message_or(&self, fallback: &str) -> String {
        let mut chars = self.message().unwrap_or_default().chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => fallback.to_string(),
        }
    }

    /// True only for a terminal `401` — the one predicate a "session expired" banner should gate
    /// on.
    #[cfg(test)]
    pub fn is_session_expired(&self) -> bool {
        matches!(self, Self::SessionExpired { .. })
    }
}
