//! Source pins for the results transaction's live wiring: the retract-before-derive attendance
//! order, and the leaderboard refresh as the last statement before commit.
//!
//! Helper-only suites stay green when a call site is deleted, so these assert on stripped
//! service source. The attendance statements live in `participation_attribution`, so the retract
//! pin windows that file too — the plumbing and the SQL have to be pinned together or a deletion
//! on either side false-greens.

use crate::match_telemetry::services::ingest_parsing::tests::{
    collapse_ws, production_half, strip_rust_comments,
};

const INGEST_SRC: &str = include_str!("../match_results_ingest.rs");
const ATTENDANCE_SRC: &str =
    include_str!("../../../operations/services/participation_attribution.rs");

/// The `ingest_results_revision` body, comments stripped and whitespace collapsed.
fn ingest_transaction() -> String {
    let production = production_half(INGEST_SRC);
    let start = production
        .find("pub async fn ingest_results_revision")
        .expect("ingest_results_revision must exist");
    let after = &production[start..];
    let end = after[1..]
        .find("\npub async fn ")
        .map(|i| i + 1)
        .unwrap_or(after.len());
    collapse_ws(&strip_rust_comments(&after[..end]))
}

/// The applied revision reconciles provenance on its open transaction before deriving
/// statistics, and refreshes the leaderboard (its advisory lock is always last) before commit.
#[test]
fn ingest_results_revision_retracts_prior_attendance_before_deriving_statistics() {
    let compact: String = ingest_transaction()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    assert!(compact.contains("reconcile_match(&muttx,stored.id,&affected).await?"));
    let reconcile = compact.find("reconcile_match(").unwrap();
    let recompute = compact.find("recompute_user_stats_on_connection(").unwrap();
    let refresh = compact.find("refresh_leaderboard_on_connection(").unwrap();
    let commit = compact.rfind("tx.commit()").unwrap();
    assert!(reconcile < recompute && recompute < refresh && refresh < commit);
    let attendance = strip_rust_comments(ATTENDANCE_SRC);
    assert!(attendance.contains("DELETE FROM event_registration_participation"));
    assert!(attendance.contains("m.event_id = em.event_id AND m.mission_id = em.mission_id"));
    assert!(attendance.contains("m.finalized_at IS NOT NULL"));
    assert!(!attendance.contains("SET reservation_state"));
}

/// Nothing is decided before the match row lock: the revision decision reads the locked row.
#[test]
fn ingest_results_revision_decides_under_the_match_row_lock() {
    let compact: String = ingest_transaction()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    let lock = compact.find("lock_registered_match(").unwrap();
    let decide = compact.find("decide_revision(").unwrap();
    let identities = compact.find("lock_identities(").unwrap();
    assert!(lock < decide && decide < identities);
}
