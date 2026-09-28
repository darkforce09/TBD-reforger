//! Failure injection at the commit boundary of a results revision.
//!
//! **Role:** proves that a results revision the ingest fails before its commit counts nothing and
//! a clean retry applies it, and that a revision whose answer is lost after the commit is counted
//! once: the retry of the same body is an inert duplicate.
//! **Position:** its own test binary over `POST /api/v1/ingest/match-results`, driven through the
//! real router with a registered server's machine credential (`tests/telemetry_support`); the
//! failpoints `ResultsRevisionBeforeCommit` and `ResultsRevisionAfterCommit` sit in
//! `match_telemetry::services::match_results_ingest::ingest_results_revision`.
//! **Signals & state:** the process-global failpoint registry, serialised by the suite lock every
//! case holds for its whole body; this binary's private database from `tests/common`.
//! **Invariants:** a failed-before-commit revision leaves the match, its player lines, the derived
//! statistics and the audit trail exactly as they were; a committed revision is counted once
//! however often its report is retried.

mod common;
mod failpoint_and_race_support;
mod telemetry_support;

use axum::Router;
use failpoint_and_race_support::{
    AuditEvidence, Failpoint, FailpointArming, RowCounts, assert_injected_failure, audit_evidence,
    lock_suite,
};
use serde_json::Value;
use sqlx::PgPool;
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{
    boot_with_state, counters, leaderboard_kills, line, match_state, player_rows, report,
    seed_player,
};
use uuid::Uuid;

/// The audit an applied revision writes for its unlinked player lines.
const UNLINKED_AUDIT: &str = "match.unlinked_players";

/// The tables an applied revision writes, whole: a revision failed before its commit leaves
/// every count unchanged.
const REVISION_TABLES: [&str; 4] = [
    "match_player_stats",
    "leaderboard_totals",
    "audit_logs",
    "audit_publication_pending",
];

const NOTHING_AUDITED: AuditEvidence = AuditEvidence {
    rows: 0,
    pending: 0,
    published: 0,
};

/// One registered match with a linked player (4 kills, 1 death) and an unlinked one.
struct ReportedMatch {
    app: Router,
    pool: PgPool,
    server: ReportingServer,
    match_id: Uuid,
    discord_id: String,
    body: Value,
}

impl ReportedMatch {
    async fn register(case: &str) -> Self {
        let (app, pool, _state) = boot_with_state().await;
        let server = ReportingServer::open(&app, &pool, &common::unique_arma(case)).await;
        let (discord_id, _username, arma) = seed_player(&pool, case).await;
        let source = common::unique_arma(&format!("{case}-match"));
        let match_id = server.register_match(&app, &source).await;
        let unlinked = common::unique_arma(&format!("{case}-unlinked"));
        let body = report(
            &source,
            "success",
            vec![
                line(&arma, "life-1", Some(counters(4, 1))),
                line(&unlinked, "life-2", Some(counters(2, 0))),
            ],
        );
        Self {
            app,
            pool,
            server,
            match_id,
            discord_id,
            body,
        }
    }

    async fn post_first_revision(&self) -> (axum::http::StatusCode, Value) {
        self.server.post_results(&self.app, 1, &self.body).await
    }

    async fn unlinked_audit(&self) -> AuditEvidence {
        audit_evidence(&self.pool, UNLINKED_AUDIT, &self.match_id.to_string()).await
    }
}

#[tokio::test]
async fn failure_injection_results_revision_before_commit_counts_nothing_and_the_retry_applies() {
    let suite = lock_suite().await;
    let reported = ReportedMatch::register("fi-results-before").await;
    let pool = &reported.pool;
    let stored_before = match_state(pool, reported.match_id).await;
    let counted_before = leaderboard_kills(pool, &reported.discord_id).await;
    let tables_before = RowCounts::capture(pool, &REVISION_TABLES).await;

    {
        let guard = suite.fail(Failpoint::ResultsRevisionBeforeCommit);
        let (status, body) = reported.post_first_revision().await;
        assert_injected_failure(status, &body, Failpoint::ResultsRevisionBeforeCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    assert_eq!(
        match_state(pool, reported.match_id).await,
        stored_before,
        "the match row rolled back"
    );
    assert!(player_rows(pool, reported.match_id).await.is_empty());
    assert_eq!(
        leaderboard_kills(pool, &reported.discord_id).await,
        counted_before,
        "nothing was counted"
    );
    assert_eq!(reported.unlinked_audit().await, NOTHING_AUDITED);
    tables_before.check_unchanged(pool).await.unwrap();

    let (status, applied) = reported.post_first_revision().await;
    assert_eq!(status, axum::http::StatusCode::OK, "{applied}");
    assert_eq!(applied["applied"], true);
    assert_eq!(applied["revision"], 1);
    assert_eq!(match_state(pool, reported.match_id).await.0, 1);
    assert_eq!(player_rows(pool, reported.match_id).await.len(), 2);
    assert_eq!(
        leaderboard_kills(pool, &reported.discord_id).await,
        (Some(4), 1)
    );
    assert_eq!(reported.unlinked_audit().await.rows, 1);
}

#[tokio::test]
async fn failure_injection_results_revision_after_commit_retry_is_an_inert_duplicate_counted_once()
{
    let suite = lock_suite().await;
    let reported = ReportedMatch::register("fi-results-after").await;
    let pool = &reported.pool;

    {
        let guard = suite.fail(Failpoint::ResultsRevisionAfterCommit);
        let (status, body) = reported.post_first_revision().await;
        assert_injected_failure(status, &body, Failpoint::ResultsRevisionAfterCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    // The answer was lost, the revision was not: it committed with its derived statistics.
    let committed = match_state(pool, reported.match_id).await;
    let lines = player_rows(pool, reported.match_id).await;
    assert_eq!(committed.0, 1);
    assert_eq!(lines.len(), 2);
    assert_eq!(
        leaderboard_kills(pool, &reported.discord_id).await,
        (Some(4), 1)
    );
    assert_eq!(reported.unlinked_audit().await.rows, 1);
    let tables_committed = RowCounts::capture(pool, &REVISION_TABLES).await;

    let (status, retry) = reported.post_first_revision().await;
    assert_eq!(status, axum::http::StatusCode::OK, "{retry}");
    assert_eq!(retry["applied"], false, "the retry is a duplicate: {retry}");
    assert_eq!(retry["revision"], 1);
    assert_eq!(
        retry["match_id"],
        reported.match_id.to_string(),
        "the duplicate answers the committed match"
    );
    assert_eq!(match_state(pool, reported.match_id).await, committed);
    assert_eq!(player_rows(pool, reported.match_id).await, lines);
    assert_eq!(
        leaderboard_kills(pool, &reported.discord_id).await,
        (Some(4), 1),
        "the kills and the deployment are counted once"
    );
    assert_eq!(reported.unlinked_audit().await.rows, 1);
    tables_committed.check_unchanged(pool).await.unwrap();
}
