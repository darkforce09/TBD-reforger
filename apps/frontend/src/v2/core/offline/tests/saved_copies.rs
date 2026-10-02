//! Tests for [`super`] — where a successful read came from.

use super::*;

#[test]
fn an_unmarked_answer_came_from_the_server_whatever_its_date() {
    assert_eq!(source_of_answer(false, None), ReadSource::Server);
    assert_eq!(
        source_of_answer(false, Some("Sun, 28 Sep 2026 14:05:09 GMT")),
        ReadSource::Server
    );
}

#[test]
fn a_marked_answer_is_the_saved_copy_dated_by_its_date_header() {
    assert_eq!(
        source_of_answer(true, Some("Sun, 28 Sep 2026 14:05:09 GMT")),
        ReadSource::SavedCopy {
            saved_on: Some("28 Sep 2026, 14:05 UTC".into())
        }
    );
    assert_eq!(
        source_of_answer(true, None),
        ReadSource::SavedCopy { saved_on: None }
    );
    assert_eq!(
        source_of_answer(true, Some("not a date")),
        ReadSource::SavedCopy { saved_on: None }
    );
}

#[test]
fn the_marker_is_the_header_the_offline_worker_adds() {
    assert_eq!(
        SAVED_COPY_HEADER,
        offline_cache_policy::network_fallback::SAVED_COPY_HEADER
    );
}
