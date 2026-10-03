//! The decision a results revision meets against the stored match.
//!
//! **Role:** classifies an incoming revision as applied, an inert retry, a conflict, stale, or a
//! forbidden return of a finalized match to pending.
//! **Position:** pure policy called by [`super::match_results_ingest`] under the match row lock.
//! **Signals & state:** none; pure functions.
//! **Invariants:** only a strictly higher revision changes facts; the same revision is inert with
//! the same digest and a conflict with another; a lower revision is stale; a finalized match never
//! returns to `pending`.

/// What happens to an incoming revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionDecision {
    /// A higher revision that changes the stored facts.
    Apply,
    /// The stored revision again, with the same digest: nothing changes.
    Duplicate,
    /// The stored revision again, with another digest.
    Conflict,
    /// Older than the stored revision.
    Stale,
    /// A higher revision that would return a finalized match to `pending`.
    FinalizedRegression,
}

/// Decide an incoming revision against the stored `stored_revision` and `stored_sha256`.
pub fn decide_revision(
    stored_revision: i64,
    stored_sha256: Option<&str>,
    finalized: bool,
    incoming_revision: i64,
    incoming_sha256: &str,
    incoming_is_pending: bool,
) -> RevisionDecision {
    if incoming_revision < stored_revision {
        RevisionDecision::Stale
    } else if incoming_revision == stored_revision {
        if stored_sha256 == Some(incoming_sha256) {
            RevisionDecision::Duplicate
        } else {
            RevisionDecision::Conflict
        }
    } else if finalized && incoming_is_pending {
        RevisionDecision::FinalizedRegression
    } else {
        RevisionDecision::Apply
    }
}

#[cfg(test)]
#[path = "tests/results_revision.rs"]
mod tests;
