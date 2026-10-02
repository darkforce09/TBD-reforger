//! The short texts the wiki derives for display: a calendar day, a revision's byline, a load
//! failure.
//!
//! **Role:** formats the day of an ISO timestamp, the "saved … by …" line of a revision, and the
//! sentence shown when a manual, its history or a revision fails to load.
//! **Position:** read by the article header, the revision list and the revision view.
//! **Signals & state:** none; pure functions.
//! **Invariants:** the day is the timestamp's own `YYYY-MM-DD` prefix, never re-zoned; a text
//! that is not an ISO timestamp reads as an em dash. A refused session is named as such, never
//! as a missing manual.

use crate::v2::core::api::client::ApiFailure;

/// The `YYYY-MM-DD` prefix of an ISO timestamp, or an em dash when there is none.
///
/// String slicing rather than a date library: this runs in the native unit tests as well as in
/// the browser.
pub(super) fn calendar_day(iso: &str) -> String {
    let day = iso.get(..10).unwrap_or("");
    if day.len() == 10 && day.as_bytes().get(4) == Some(&b'-') {
        day.to_string()
    } else {
        "—".into()
    }
}

/// `saved <day>`, then ` by <editor>` when the editor is known.
pub(super) fn revision_byline(created_at: &str, author_id: Option<&str>) -> String {
    let day = calendar_day(created_at);
    match author_id.filter(|author| !author.is_empty()) {
        Some(author) => format!("saved {day} by {author}"),
        None => format!("saved {day}"),
    }
}

/// The sentence shown when `what` (such as "this manual") failed to load for `failure`.
pub(super) fn load_failure_text(failure: &ApiFailure, what: &str) -> String {
    match failure {
        ApiFailure::SessionExpired { .. } => {
            "Your session has ended. Sign in again to read the manuals.".to_string()
        }
        ApiFailure::Http { status: 404, .. } => format!("Could not find {what}."),
        _ => format!("Failed to load {what}."),
    }
}

#[cfg(test)]
#[path = "tests/display_text.rs"]
mod tests;
