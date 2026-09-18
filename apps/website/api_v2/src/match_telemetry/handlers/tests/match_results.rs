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
const ATTENDANCE_SRC: &str = include_str!("../attendance_attribution.rs");
const UPSERT_SRC: &str = include_str!("../match_upsert.rs");

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

/// Class-R: both `ingest_match_results` sites must invoke `require_role_played`.
///
/// Helper-only tests (`blank_role_played_is_rejected` / `non_blank_role_played_ok`) stay green
/// if the call sites are deleted — this pin fails that deletion. Comments are stripped before
/// counting so a bait comment cannot false-green a deleted live call.
#[test]
fn ingest_match_results_invokes_require_role_played_at_both_sites() {
    let collapsed = results_handler();
    // Assembled so a free-floating bait comment / this test's source cannot false-green
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

/// Class-R: a re-point must retract prior event_mission attendance before the SET.
///
/// A SET-only path false-greens every "marks EV2" assert while leaving EV1 attended. The full
/// integration coverage lives in `tests/telemetry.rs`; this pin fails if the retract UPDATE, its
/// NOT EXISTS attribution guard, or the `retract_from` plumbing is deleted.
#[test]
fn ingest_match_results_retracts_prior_attendance_on_repoint() {
    let collapsed = results_handler();
    assert!(
        collapsed.contains("let (match_id, retract_from) = upsert_match("),
        "ingest must capture upsert_match's retract_from"
    );
    assert!(
        collapsed.contains("if let Some((old_event, old_mission)) = retract_from"),
        "ingest must act on retract_from before the attended SET"
    );
    let retract_call = collapsed
        .find("retract_prior_attendance(&mut tx,")
        .expect("ingest must call retract_prior_attendance on the open transaction");
    let mark_call = collapsed
        .find("mark_attended(&mut tx,")
        .expect("ingest must call mark_attended on the open transaction");
    assert!(
        retract_call < mark_call,
        "the retract must run before the attended SET, on the same transaction"
    );

    let attendance = collapse_ws(&strip_rust_comments(production_half(ATTENDANCE_SRC)));
    assert!(
        attendance.contains("UPDATE event_registrations er SET state = 'registered'"),
        "re-point must retract prior attendance to registered"
    );
    assert!(
        attendance.contains(
            "AND NOT EXISTS ( \\ SELECT 1 FROM matches m \\ INNER JOIN match_player_stats mps"
        ),
        "retract must keep attendance when another match still attributes the player"
    );
    assert!(
        attendance.contains("WHERE m.id <> $4 \\ AND m.event_id = $2 \\ AND m.mission_id = $3"),
        "NOT EXISTS must exclude this match and key the prior (event_id, mission_id) pair"
    );

    let upsert_production = production_half(UPSERT_SRC);
    let up_start = upsert_production
        .find("async fn upsert_match")
        .expect("upsert_match must exist");
    let up_after = &upsert_production[up_start..];
    let up_end = up_after[1..]
        .find("\nasync fn ")
        .or_else(|| up_after[1..].find("\npub(super) async fn "))
        .map(|i| i + 1)
        .unwrap_or(up_after.len());
    let up_collapsed = collapse_ws(&strip_rust_comments(&up_after[..up_end]));
    assert!(
        up_collapsed
            .contains("SELECT id, event_id, mission_id FROM matches WHERE source_match_id = $1"),
        "re-ingest must read the prior (event_id, mission_id) before COALESCE UPDATE"
    );
    assert!(
        up_collapsed.contains("RETURNING event_id, mission_id"),
        "re-ingest must RETURN the merged pair so retract_from can detect a move"
    );
    assert!(
        up_collapsed.contains("Ok((row.0, None))"),
        "create path must return no retract_from"
    );
}
