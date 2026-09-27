//! Recovery of the audit delivery stream from database failures that leave the listener healthy.
//!
//! A read failure is injected in this binary's private database by renaming `audit_logs`, the
//! relation the replay read joins: publication (which touches only the publication tables) and
//! `LISTEN audit_log` keep working, and a row planted in the renamed table still raises its
//! notification and its pending entry. A publish failure is injected by a statement trigger that
//! refuses every insert into `audit_publications` and counts its refusals in a sequence, which a
//! rollback does not undo. The cases share one database, so they run one at a time behind the
//! suite lock, and each first heals whatever a failed predecessor left injected.
//!
//! ## What makes this fail (non-vacuity)
//! A wake that stops at a failed publication instead of going on to the read turns
//! `audit_query_recovery_publish_failure_still_reads_published_rows` red.

mod audit_stream_support;
mod common;
mod contract_support;

use std::time::Duration;

use futures::{Stream, StreamExt};
use sqlx::PgPool;
use tokio::sync::broadcast::error::TryRecvError;
use website_api::administration::services::audit_delivery::{
    AuditStreamItem, audit_delivery_stream,
};
use website_api::administration::services::audit_notifier::{AuditNotify, AuditSignal};

use audit_stream_support::{
    AUDIT_SCHEMA, AuditHarness, WITHHELD_AUDIT_TABLE, case_tag, drive, next_item, plant_row,
    plant_row_in, plant_rows, plant_rows_in, publication_bounds, publications_after, publish_all,
    sequence_of, serialise_case, wait_published, wait_signal,
};

const SUITE: &str = "audit_query_recovery";

/// The replay timer of the service-level cases.
const TICK: Duration = Duration::from_millis(200);

/// The bound on any item a case expects.
const BOUND: Duration = Duration::from_secs(5);

/// Long enough for several timer ticks, each of which retries the failing read.
const OUTAGE: Duration = Duration::from_secs(1);

/// Restores the audit table and removes the publication refusal, whichever is present.
async fn heal(pool: &PgPool) {
    sqlx::raw_sql(
        "DO $$ BEGIN \
           IF to_regclass('public.audit_logs_withheld') IS NOT NULL THEN \
             ALTER TABLE public.audit_logs_withheld RENAME TO audit_logs; \
           END IF; \
         END $$; \
         DROP TRIGGER IF EXISTS audit_query_recovery_refuse_publication \
           ON public.audit_publications;",
    )
    .execute(pool)
    .await
    .expect("heal the injected failures");
}

/// Makes every replay read fail: the relation it joins is renamed away.
async fn withhold_audit_table(pool: &PgPool) {
    sqlx::raw_sql("ALTER TABLE public.audit_logs RENAME TO audit_logs_withheld")
        .execute(pool)
        .await
        .expect("withhold audit_logs");
}

async fn restore_audit_table(pool: &PgPool) {
    sqlx::raw_sql("ALTER TABLE public.audit_logs_withheld RENAME TO audit_logs")
        .execute(pool)
        .await
        .expect("restore audit_logs");
}

/// Makes every publication fail, counting each refused attempt.
async fn refuse_publication(pool: &PgPool) {
    sqlx::raw_sql(
        "CREATE SEQUENCE IF NOT EXISTS public.audit_query_recovery_refusals; \
         CREATE OR REPLACE FUNCTION public.audit_query_recovery_refuse() RETURNS trigger \
         LANGUAGE plpgsql AS $$ BEGIN \
           PERFORM nextval('public.audit_query_recovery_refusals'); \
           RAISE EXCEPTION 'injected audit publication failure'; \
         END $$; \
         CREATE TRIGGER audit_query_recovery_refuse_publication \
           BEFORE INSERT ON public.audit_publications \
           FOR EACH STATEMENT EXECUTE FUNCTION public.audit_query_recovery_refuse();",
    )
    .execute(pool)
    .await
    .expect("refuse publication");
}

async fn allow_publication(pool: &PgPool) {
    sqlx::raw_sql(
        "DROP TRIGGER audit_query_recovery_refuse_publication ON public.audit_publications",
    )
    .execute(pool)
    .await
    .expect("allow publication");
}

/// Refused publication attempts so far.
async fn refusals(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT CASE WHEN is_called THEN last_value ELSE 0 END \
         FROM public.audit_query_recovery_refusals",
    )
    .fetch_one(pool)
    .await
    .expect("read the refusal count")
}

fn assert_listener_unchanged(notify: &AuditNotify, pid: u32) {
    assert!(notify.is_listening(), "the listener stays up");
    assert_eq!(notify.backend_pid(), Some(pid), "on the same backend");
}

/// The next item, which must be the delivery of `audit_id` under `sequence`.
fn assert_delivery(item: Option<AuditStreamItem>, sequence: i64, audit_id: i64, why: &str) {
    match item {
        Some(AuditStreamItem::Delivery(delivery)) => {
            assert_eq!(delivery.sequence, sequence, "{why}");
            assert_eq!(delivery.row.id, audit_id, "{why}");
            let data = serde_json::to_value(&delivery.row).expect("row JSON");
            contract_support::assert_valid(AUDIT_SCHEMA, Some("AuditLogEntry"), &data);
        }
        other => panic!("expected the delivery of {audit_id} at {sequence} ({why}), got {other:?}"),
    }
}

/// Polls `items` for its next item within `bound`; `None` when none comes in time.
async fn poll_within<S: Stream + Unpin>(items: &mut S, bound: Duration) -> Option<S::Item> {
    tokio::time::timeout(bound, items.next())
        .await
        .ok()
        .flatten()
}

/// Opens a service-level stream at the tail and answers its `ready` cursor.
async fn open_at_tail(
    harness: &AuditHarness,
) -> (tokio::sync::mpsc::UnboundedReceiver<AuditStreamItem>, i64) {
    let items = audit_delivery_stream(harness.pool.clone(), harness.notify.clone(), TICK, None)
        .await
        .expect("open the delivery stream");
    let mut items = drive(items);
    match next_item(&mut items, BOUND).await {
        Some(AuditStreamItem::Ready(ready)) => (items, ready.resume_after),
        other => panic!("expected ready, got {other:?}"),
    }
}

#[tokio::test]
async fn audit_query_recovery_read_failure_is_retried_by_the_timer_while_the_listener_stays_up() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    heal(pool).await;
    publish_all(pool).await;
    let pid = harness.notify.backend_pid().expect("listening");
    let (mut items, tail) = open_at_tail(&harness).await;
    let tag = case_tag("read-retry");

    withhold_audit_table(pool).await;
    let mut probe = harness.notify.subscribe();
    let row = plant_row_in(pool, WITHHELD_AUDIT_TABLE, &tag, 0).await;
    // Only the stream publishes in this binary: the notification woke it and it published.
    let sequence = wait_published(pool, row, BOUND).await;
    assert!(sequence > tail);
    wait_signal(
        &mut probe,
        AuditSignal::Row(row),
        BOUND,
        "the insert's notification",
    )
    .await;
    wait_signal(
        &mut probe,
        AuditSignal::Row(sequence),
        BOUND,
        "the publication's",
    )
    .await;
    assert!(
        next_item(&mut items, OUTAGE).await.is_none(),
        "no delivery while every read fails"
    );
    assert_listener_unchanged(&harness.notify, pid);

    let mut after_restore = harness.notify.subscribe();
    restore_audit_table(pool).await;
    let item = next_item(&mut items, BOUND).await;
    assert_delivery(item, sequence, row, "the timer's retry after the restore");
    assert!(
        matches!(after_restore.try_recv(), Err(TryRecvError::Empty)),
        "no notification preceded the delivery: the timer delivered it"
    );
    assert_listener_unchanged(&harness.notify, pid);
}

#[tokio::test]
async fn audit_query_recovery_publish_failure_still_reads_published_rows() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    heal(pool).await;
    publish_all(pool).await;
    let items = audit_delivery_stream(pool.clone(), harness.notify.clone(), TICK, None)
        .await
        .expect("open the delivery stream");
    // Polled by hand: after `ready` the stream runs only when the case reads it.
    let mut items = Box::pin(items);
    let ready = poll_within(&mut items, BOUND).await;
    assert!(
        matches!(ready, Some(AuditStreamItem::Ready(_))),
        "{ready:?}"
    );
    let tag = case_tag("publish-failure");

    let published = plant_rows(pool, &tag, 2).await;
    publish_all(pool).await;
    let stuck = plant_row(pool, &tag, 2).await;
    refuse_publication(pool).await;
    let refused_before = refusals(pool).await;

    for audit_id in &published {
        let sequence = sequence_of(pool, *audit_id).await.expect("published");
        let item = poll_within(&mut items, BOUND).await;
        assert_delivery(item, sequence, *audit_id, "published before the failure");
    }
    assert!(
        refusals(pool).await > refused_before,
        "the stream attempted the publication, and it failed"
    );
    assert_eq!(
        sequence_of(pool, stuck).await,
        None,
        "the refused row stays pending"
    );

    allow_publication(pool).await;
    let item = poll_within(&mut items, BOUND).await;
    let sequence = sequence_of(pool, stuck)
        .await
        .expect("published after the refusal ends");
    assert_delivery(item, sequence, stuck, "published on a later wake");
}

#[tokio::test]
async fn audit_query_recovery_failed_read_keeps_the_cursor_without_loss_or_duplicate() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    heal(pool).await;
    publish_all(pool).await;
    let pid = harness.notify.backend_pid().expect("listening");
    let (mut items, tail) = open_at_tail(&harness).await;
    let tag = case_tag("keep-cursor");

    let before = plant_row(pool, &tag, 0).await;
    let first = wait_published(pool, before, BOUND).await;
    assert_delivery(
        next_item(&mut items, BOUND).await,
        first,
        before,
        "before the outage",
    );

    withhold_audit_table(pool).await;
    let during = plant_rows_in(pool, WITHHELD_AUDIT_TABLE, &tag, 3).await;
    for audit_id in &during {
        wait_published(pool, *audit_id, BOUND).await;
    }
    assert!(next_item(&mut items, OUTAGE).await.is_none(), "reads fail");
    assert_listener_unchanged(&harness.notify, pid);
    restore_audit_table(pool).await;

    let outage_rows = publications_after(pool, first).await;
    assert_eq!(outage_rows.len(), 3);
    for (sequence, audit_id) in &outage_rows {
        assert_delivery(
            next_item(&mut items, BOUND).await,
            *sequence,
            *audit_id,
            "after",
        );
    }
    // The next delivery is the next row: nothing was delivered twice.
    let sentinel = plant_row(pool, &tag, 4).await;
    let sentinel_sequence = wait_published(pool, sentinel, BOUND).await;
    assert_delivery(
        next_item(&mut items, BOUND).await,
        sentinel_sequence,
        sentinel,
        "next",
    );
    let all = publications_after(pool, tail).await;
    let ids: Vec<i64> = all.iter().map(|publication| publication.1).collect();
    assert_eq!(
        ids,
        [vec![before], during, vec![sentinel]].concat(),
        "no loss"
    );
    assert!(
        next_item(&mut items, TICK * 3).await.is_none(),
        "no duplicate"
    );
    assert_listener_unchanged(&harness.notify, pid);
}

#[tokio::test]
async fn audit_query_recovery_http_stream_opened_during_an_outage_delivers_after_restore() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    heal(pool).await;
    publish_all(pool).await;
    let pid = harness.notify.backend_pid().expect("listening");
    let (cursor, floor) = publication_bounds(pool).await;
    let tag = case_tag("http-outage");
    let planted = plant_rows(pool, &tag, 2).await;
    publish_all(pool).await;
    let expected = publications_after(pool, cursor).await;
    assert_eq!(expected.iter().map(|p| p.1).collect::<Vec<_>>(), planted);

    // The opening reads only the bounds, so the stream opens while every row read fails.
    withhold_audit_table(pool).await;
    let mut stream = harness.open_stream(Some(cursor)).await;
    let ready = stream.expect_event(BOUND, "ready during the outage").await;
    assert_eq!(ready.event.as_deref(), Some("ready"));
    assert_eq!(ready.sequence(), cursor);
    contract_support::assert_valid(AUDIT_SCHEMA, Some("AuditStreamReady"), &ready.json());
    assert_eq!(ready.json()["retained_after"], floor);
    stream
        .expect_quiet(Duration::from_millis(2500), "no row while the read fails")
        .await;
    assert_listener_unchanged(&harness.notify, pid);

    let mut after_restore = harness.notify.subscribe();
    restore_audit_table(pool).await;
    let rows = stream
        .expect_rows(2, BOUND, "the rows, on the next timer tick")
        .await;
    for (row, (sequence, audit_id)) in rows.iter().zip(&expected) {
        assert_eq!(row.sequence(), *sequence);
        let data = row.json();
        contract_support::assert_valid(AUDIT_SCHEMA, Some("AuditLogEntry"), &data);
        assert_eq!(data["id"], *audit_id);
    }
    assert!(
        matches!(after_restore.try_recv(), Err(TryRecvError::Empty)),
        "no notification preceded the delivery: the timer delivered it"
    );
    assert_listener_unchanged(&harness.notify, pid);
}
