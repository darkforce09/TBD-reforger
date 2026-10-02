//! The wiki's derived texts: the calendar day, a revision's byline, a load failure.

use super::*;

#[test]
fn updated_day_takes_iso_prefix() {
    assert_eq!(calendar_day("2026-07-14T10:12:00Z"), "2026-07-14");
    assert_eq!(calendar_day(""), "—");
    assert_eq!(calendar_day("yesterday"), "—");
}

#[test]
fn wiki_revision_byline_names_the_day_and_a_known_editor() {
    assert_eq!(
        revision_byline("2026-07-14T10:12:00Z", Some("000000000000000001")),
        "saved 2026-07-14 by 000000000000000001"
    );
    assert_eq!(
        revision_byline("2026-07-14T10:12:00Z", None),
        "saved 2026-07-14"
    );
    assert_eq!(
        revision_byline("2026-07-14T10:12:00Z", Some("")),
        "saved 2026-07-14"
    );
}

#[test]
fn wiki_load_failure_names_the_session_the_missing_item_or_the_failure() {
    assert_eq!(
        load_failure_text(&ApiFailure::SessionExpired { message: None }, "this manual"),
        "Your session has ended. Sign in again to read the manuals."
    );
    assert_eq!(
        load_failure_text(
            &ApiFailure::Http {
                status: 404,
                message: None
            },
            "this revision"
        ),
        "Could not find this revision."
    );
    assert_eq!(
        load_failure_text(&ApiFailure::Transport, "the revision history"),
        "Failed to load the revision history."
    );
}
