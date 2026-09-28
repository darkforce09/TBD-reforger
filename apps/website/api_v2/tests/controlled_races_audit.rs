//! Controlled race of audit appends whose transactions commit in inverted id order.
//!
//! **Role:** proves that the publication sequence follows publisher commits, not audit ids: two
//! appends hold ids `lower < higher`, commit in either order while a publisher is paused inside
//! its batch, and every row is published exactly once at contiguous sequences and delivered by
//! the audit stream once, in sequence order; a client whose cursor already passed the first
//! delivered row still receives the other one when it resumes.
//! **Position:** appends through `required_audit::append_system_audit`, publishes through
//! `audit_publication::publish_audit_batch` and reads `GET /api/v1/admin/audit-logs/stream`
//! through `tests/audit_stream_support`; orders the publishers with the failpoint pause
//! `AuditPublicationBeforeCommit` and the wait until a publisher queues behind the paused one's
//! lock on `audit_publication_state` (`tests/failpoint_and_race_support`).
//! **Signals & state:** one per-binary database, so the publication sequence belongs to this
//! suite; the case holds the failpoint suite lock for its whole body and reads each stream only
//! after the publishers finish, so the stream's own publisher never races the paused one.
//! **Invariants:** in each interleaving the paused publisher holds exactly the first committed
//! row, the second row commits while it is paused, and a queued publisher publishes that row at
//! the next sequence after the release.

mod audit_stream_support;
mod common;
mod failpoint_and_race_support;

use std::time::Duration;

use sqlx::PgConnection;
use website_api::administration::services::audit_publication::publish_audit_batch;
use website_api::administration::services::required_audit::append_system_audit;

use audit_stream_support::{AuditHarness, SseEvent, SseReader, case_tag, sequence_of};
use failpoint_and_race_support::{
    AuditEvidence, Failpoint, FailpointArming, Interleaving, audit_evidence,
    check_publication_sequence, lock_suite, paused_transaction_backend, run_in_both_orders,
    wait_for_blocked,
};

const SUITE: &str = "controlled_races_audit";

/// The bound on any event the case expects.
const BOUND: Duration = Duration::from_secs(10);

/// How long a stream stays open to prove nothing else arrives.
const QUIET: Duration = Duration::from_millis(500);

/// The largest batch a publisher of this suite asks for.
const PUBLISH_BATCH: i64 = 1000;

/// Appends one system audit row on `connection` and answers its id, which the open transaction
/// already sees.
async fn append(connection: &mut PgConnection, tag: &str, target: &str) -> i64 {
    append_system_audit(
        connection,
        tag,
        "audit_stream",
        target,
        "controlled race audit append",
    )
    .await
    .expect("append the audit row");
    sqlx::query_scalar("SELECT id FROM audit_logs WHERE action = $1 AND target_id = $2")
        .bind(tag)
        .bind(target)
        .fetch_one(connection)
        .await
        .expect("read the appended row's id")
}

/// Opens the stream (at the tail, or after `resume_after`) and consumes its `ready` event.
async fn open_after_ready(harness: &AuditHarness, resume_after: Option<i64>) -> SseReader {
    let mut stream = harness.open_stream(resume_after).await;
    let ready = stream
        .expect_event(BOUND, "the stream opens with ready")
        .await;
    assert_eq!(ready.event.as_deref(), Some("ready"), "{ready:?}");
    stream
}

/// `(publication sequence, audit id)` of each delivered row.
fn delivered(rows: &[SseEvent]) -> Vec<(i64, i64)> {
    rows.iter()
        .map(|row| {
            let id = row.json()["id"]
                .as_i64()
                .unwrap_or_else(|| panic!("a delivered row carries its id: {row:?}"));
            (row.sequence(), id)
        })
        .collect()
}

/// One appended row: its id and the target that names it in `audit_logs`.
#[derive(Clone, Copy)]
struct Appended {
    id: i64,
    target: &'static str,
}

#[tokio::test]
async fn controlled_races_audit_inverted_commit_order_publishes_and_delivers_every_row_once() {
    let suite = lock_suite().await;
    let suite = &suite;
    let harness = AuditHarness::boot(SUITE).await;
    let harness = &harness;
    let pool = &harness.pool;
    let [natural, inverted] = run_in_both_orders(|order| async move {
        let mut stream = open_after_ready(harness, None).await;
        let tag = case_tag(SUITE);

        let mut lower_transaction = pool.begin().await.expect("begin the lower append");
        let lower = append(&mut lower_transaction, &tag, "lower").await;
        let mut higher_transaction = pool.begin().await.expect("begin the higher append");
        let higher = append(&mut higher_transaction, &tag, "higher").await;
        assert!(lower < higher, "the first append holds the lower id");
        let ((leading_transaction, leader), (following_transaction, follower)) = order.arrange(
            (
                lower_transaction,
                Appended {
                    id: lower,
                    target: "lower",
                },
            ),
            (
                higher_transaction,
                Appended {
                    id: higher,
                    target: "higher",
                },
            ),
        );

        leading_transaction
            .commit()
            .await
            .expect("commit the leading append");
        let paused = suite.pause(Failpoint::AuditPublicationBeforeCommit);
        let publishing_leader = tokio::spawn({
            let pool = pool.clone();
            async move { publish_audit_batch(&pool, PUBLISH_BATCH).await }
        });
        paused.reached().await;
        following_transaction
            .commit()
            .await
            .expect("commit the following append while the publisher is paused");
        let holder = paused_transaction_backend(pool).await;
        let publishing_follower = tokio::spawn({
            let pool = pool.clone();
            async move { publish_audit_batch(&pool, PUBLISH_BATCH).await }
        });
        wait_for_blocked(pool, holder, 1).await;
        paused.release();
        let leader_published = publishing_leader
            .await
            .expect("the paused publisher completes")
            .expect("the paused publisher commits");
        let follower_published = publishing_follower
            .await
            .expect("the queued publisher completes")
            .expect("the queued publisher commits");
        assert_eq!(
            (leader_published, follower_published, paused.arrivals()),
            (1, 1, 2),
            "the paused publisher publishes the leading row alone and the queued one the other"
        );
        drop(paused);

        let leader_sequence = sequence_of(pool, leader.id)
            .await
            .expect("the leading row is published");
        let follower_sequence = sequence_of(pool, follower.id)
            .await
            .expect("the following row is published");
        assert_eq!(
            follower_sequence,
            leader_sequence + 1,
            "sequences follow commits, not ids"
        );
        check_publication_sequence(pool).await.unwrap();
        for row in [leader, follower] {
            assert_eq!(
                audit_evidence(pool, &tag, row.target).await,
                AuditEvidence {
                    rows: 1,
                    pending: 0,
                    published: 1
                },
                "row {} is committed and published once",
                row.id
            );
        }

        let rows = stream
            .expect_rows(2, BOUND, "both rows reach the open stream")
            .await;
        assert_eq!(
            delivered(&rows),
            [
                (leader_sequence, leader.id),
                (follower_sequence, follower.id)
            ],
            "the stream delivers in sequence order ({})",
            order.name()
        );
        stream
            .expect_quiet(QUIET, "each row is delivered once")
            .await;

        let mut resumed = open_after_ready(harness, Some(leader_sequence)).await;
        let rows = resumed
            .expect_rows(
                1,
                BOUND,
                "the resumed stream delivers the row after its cursor",
            )
            .await;
        assert_eq!(delivered(&rows), [(follower_sequence, follower.id)]);
        resumed
            .expect_quiet(QUIET, "the resumed stream delivers nothing twice")
            .await;
        (leader.id, follower.id)
    })
    .await;
    assert!(
        natural.0 < natural.1,
        "{}: the lower id commits and is delivered first",
        Interleaving::FirstLeads.name()
    );
    assert!(
        inverted.0 > inverted.1,
        "{}: the higher id commits and is delivered first, and the lower id still follows once",
        Interleaving::SecondLeads.name()
    );
}
