//! Source pins for the results handler's live wiring: the two `require_role_played` call sites,
//! the counters fold, and the retract-before-mark attendance order.
//!
//! Helper-only suites stay green when a call site is deleted, so these assert on stripped
//! handler source. The attendance statements live in `attendance_attribution`, so the retract
//! pin windows that file too — the plumbing and the SQL have to be pinned together or a deletion
//! on either side false-greens.

use crate::match_telemetry::handlers::ingest_parsing::tests::{
    collapse_ws, production_half, strip_rust_comments,
};

const RESULTS_SRC: &str = include_str!("../match_results.rs");
const ATTENDANCE_SRC: &str =
    include_str!("../../../operations/services/participation_attribution.rs");

/// The `ingest_match_results` body, comments stripped and whitespace collapsed.
fn results_handler() -> String {
    let production = production_half(RESULTS_SRC);
    let start = production
        .find("pub async fn ingest_match_results")
        .expect("ingest_match_results handler must exist");
    let after = &production[start..];
    // The handler is the last item in its file; any sibling `async fn` added after it ends the
    // window here rather than silently widening it.
    let end = after[1..]
        .find("\nasync fn ")
        .map(|i| i + 1)
        .unwrap_or(after.len());
    collapse_ws(&strip_rust_comments(&after[..end]))
}

/// both `ingest_match_results` sites must invoke `require_role_played`.
///
/// Helper-only tests (`blank_role_played_is_rejected` / `non_blank_role_played_ok`) stay green
/// if the call sites are deleted — this pin fails that deletion. Comments are stripped before
/// counting so a comment naming the call cannot stand in for a deleted live call.
#[test]
fn ingest_match_results_invokes_require_role_played_at_both_sites() {
    let collapsed = results_handler();
    // Assembled so a comment naming the call, or this test's own source, cannot satisfy it
    // with a bare `contains("require_role_played")` on the helper-only suite.
    let call = format!("{}{}", "require_role_played(", "&p.role_played)");
    assert_eq!(
        collapsed.matches(&call).count(),
        2,
        "ingest_match_results must call require_role_played(&p.role_played) twice \
         (pre-tx roster guard + UPSERT bind); helper-only tests do not cover this"
    );
    assert!(
        collapsed.contains("p.effective_counters()"),
        "ingest must fold flat counters via effective_counters; nested still wins"
    );
}

/// The live handler reconciles provenance on its open transaction before deriving statistics.
#[test]
fn ingest_match_results_retracts_prior_attendance_on_repoint() {
    let compact: String = results_handler()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    assert!(compact.contains("reconcile_match(&muttx,match_id,&affected).await?"));
    let reconcile = compact.find("reconcile_match(").unwrap();
    let recompute = compact.find("recompute_user_stats_on_connection(").unwrap();
    assert!(reconcile < recompute);
    let attendance = strip_rust_comments(ATTENDANCE_SRC);
    assert!(attendance.contains("DELETE FROM event_registration_participation"));
    assert!(attendance.contains("m.event_id = em.event_id AND m.mission_id = em.mission_id"));
    assert!(attendance.contains("m.finalized_at IS NOT NULL"));
    assert!(!attendance.contains("SET reservation_state"));
}
