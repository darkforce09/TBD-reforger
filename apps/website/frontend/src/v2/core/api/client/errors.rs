//! Error shapes the HTTP client hands back, and the readers that turn them into prose.
//!
//! **Role:** owns the failure half of every request — the raw `(status, message)` pair, the parser
//! that lifts a backend error body into a display string, and the named failure enum the render
//! sites match on.
//! **Position:** the bottom of the API layer; every verb in this module returns one of these.
//! **Signals & state:** none. Every function is pure over its arguments.
//! **Invariants:** status `0` means the request never reached the backend. A `401` that escapes
//! this client has already survived a refresh and a retry, so it always means the session is dead
//! rather than that the route refused the caller.

/// Request failure: the HTTP status, with `0` standing for a network or deserialisation error,
/// plus the backend's `{"error": …}` string when one was sent.
///
/// The message is carried rather than flattened so that a caller can surface the backend's own
/// wording — "slot already taken" and "squad is reserved by a leader" reach the user as themselves
/// instead of as one generic failure line.
pub type ApiErr = (u16, Option<String>);

/// How many findings are folded into an error message before the tail is summarised.
///
/// The backend caps its own list; this is the second cap, sized for a dialog a person reads rather
/// than for a log.
pub const MAX_ERROR_DETAILS: usize = 6;

/// The human-readable failure inside a backend error body: the `error` string, plus the `details`
/// array when the handler sent one saying why.
///
/// Only a `details` that is an array of strings is folded in. Some routes put a structured payload
/// there for the caller to render, and that is data rather than prose. Extra findings arrive as
/// newline-separated lines so the returned string still fits [`ApiErr`]; `split_error_lines` is the
/// reader that takes them apart again.
///
/// Returns `None` when the body carries no `error` string at all.
#[allow(dead_code)]
pub fn error_body_message(body: &serde_json::Value) -> Option<String> {
    let error = body.get("error")?.as_str()?;
    let details: Vec<&str> = body
        .get("details")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<&str>>()
        })
        .unwrap_or_default();
    if details.is_empty() {
        return Some(error.to_string());
    }

    let shown = details.len().min(MAX_ERROR_DETAILS);
    let mut out = String::from(error);
    for d in &details[..shown] {
        out.push('\n');
        out.push_str(d);
    }
    if details.len() > shown {
        out.push_str(&format!("\n… and {} more", details.len() - shown));
    }
    Some(out)
}

/// Split a message built by [`error_body_message`] back into its headline and its findings.
#[allow(dead_code)]
pub fn split_error_lines(msg: Option<&str>) -> (Option<String>, Vec<String>) {
    let Some(m) = msg else {
        return (None, Vec::new());
    };
    let mut lines = m.split('\n');
    let head = lines.next().map(str::to_string);
    (head, lines.map(str::to_string).collect())
}

/// The backend's error string with its first letter capitalised, or `fallback` when the backend
/// sent no message.
#[allow(dead_code)]
pub fn api_error_message(err: &ApiErr, fallback: &str) -> String {
    match &err.1 {
        Some(msg) if !msg.is_empty() => {
            let mut c = msg.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => fallback.to_string(),
            }
        }
        _ => fallback.to_string(),
    }
}

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
/// is `403`. By the time a failure escapes [`send_with_refresh`] a `401` has already survived a
/// single-flight refresh and a retry with the rotated token.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiFailure {
    /// The session is over: refresh and retry both failed. Surface "session expired — log in
    /// again", never an empty state.
    SessionExpired {
        /// The backend's error string, when it sent one.
        message: Option<String>,
    },
    /// The backend answered and refused for a reason that is not authentication. The caller maps
    /// the status; the message is the human-readable cause.
    Http {
        /// The HTTP status the backend answered with.
        status: u16,
        /// The backend's error string, when it sent one.
        message: Option<String>,
    },
    /// The request never reached the backend — network down, blocked by CORS, or a body that would
    /// not deserialise. Distinct from [`Self::SessionExpired`] because "you are logged out" is a
    /// lie when the truth is "the network dropped".
    Transport,
}

#[allow(dead_code)]
impl ApiFailure {
    /// True only for a terminal `401` — the one predicate a "session expired" banner should gate
    /// on.
    pub fn is_session_expired(&self) -> bool {
        matches!(self, Self::SessionExpired { .. })
    }

    /// The backend's failure string, if it sent one.
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::SessionExpired { message } | Self::Http { message, .. } => message.as_deref(),
            Self::Transport => None,
        }
    }
}

/// Classify a raw request failure. Status `0` is the client's own sentinel for "never got an
/// answer", `401` is the dead session, and everything else keeps its status.
impl From<ApiErr> for ApiFailure {
    fn from((status, message): ApiErr) -> Self {
        match status {
            0 => Self::Transport,
            401 => Self::SessionExpired { message },
            _ => Self::Http { status, message },
        }
    }
}
