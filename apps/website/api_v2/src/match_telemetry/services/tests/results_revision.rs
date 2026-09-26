use super::{RevisionDecision, decide_revision};

const A: &str = "aaaa";
const B: &str = "bbbb";

#[test]
fn a_first_revision_applies_over_the_registration() {
    assert_eq!(
        decide_revision(0, None, false, 1, A, true),
        RevisionDecision::Apply
    );
}

#[test]
fn the_same_revision_and_digest_is_an_inert_retry() {
    assert_eq!(
        decide_revision(3, Some(A), true, 3, A, false),
        RevisionDecision::Duplicate
    );
}

#[test]
fn the_same_revision_with_another_digest_conflicts() {
    assert_eq!(
        decide_revision(3, Some(A), false, 3, B, false),
        RevisionDecision::Conflict
    );
}

#[test]
fn an_older_revision_is_stale_even_with_the_stored_digest() {
    assert_eq!(
        decide_revision(3, Some(A), false, 2, A, false),
        RevisionDecision::Stale
    );
}

#[test]
fn a_finalized_match_never_returns_to_pending() {
    assert_eq!(
        decide_revision(2, Some(A), true, 3, B, true),
        RevisionDecision::FinalizedRegression
    );
    assert_eq!(
        decide_revision(2, Some(A), true, 3, B, false),
        RevisionDecision::Apply
    );
    assert_eq!(
        decide_revision(2, Some(A), false, 3, B, true),
        RevisionDecision::Apply
    );
}
