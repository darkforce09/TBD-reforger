//! The shared filter predicate, on the cases the pages depend on.

use super::search_matches;

#[test]
fn empty_query_matches_all() {
    assert!(search_matches("", "anything"));
    assert!(search_matches("   ", "anything"));
}

#[test]
fn case_insensitive_substring() {
    assert!(search_matches("PLAN", "Timeline & Mission Planning"));
    assert!(search_matches("timeline", "Timeline & Mission Planning"));
    assert!(!search_matches("naval", "Timeline & Mission Planning"));
}
