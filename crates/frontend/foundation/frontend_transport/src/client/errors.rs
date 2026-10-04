//! The error body reader: the backend's `{"error": …, "details": …}` body turned into prose.
//!
//! **Role:** owns the parser that lifts a backend error body into a display string, and the reader
//! that takes that string apart again. The named failure every request returns is the crate's
//! [`crate::Error`], which carries the string this module builds.
//! **Position:** the bottom of the API layer; the request verbs and [`crate::Error::from_error_body`]
//! read every refused answer through it.
//! **Signals & state:** none. Every function is pure over its arguments.
//! **Invariants:** only a `details` array of strings is folded into the prose, at most
//! [`MAX_ERROR_DETAILS`] lines of it; a body without an `error` string yields no message.

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
/// newline-separated lines so the returned string is still one message; [`split_error_lines`] is
/// the reader that takes them apart again.
///
/// Returns `None` when the body carries no `error` string at all.
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
pub fn split_error_lines(msg: Option<&str>) -> (Option<String>, Vec<String>) {
    let Some(m) = msg else {
        return (None, Vec::new());
    };
    let mut lines = m.split('\n');
    let head = lines.next().map(str::to_string);
    (head, lines.map(str::to_string).collect())
}
