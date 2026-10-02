//! Replay and reset of `GET /api/v1/admin/audit-logs/stream`: the `ready` event, `Last-Event-ID`
//! replay in publication order, reconnects, a backlog larger than the notifier's buffer and the
//! read page, the retained floor of migration 0057, the resets it causes, and malformed cursors.
//!
//! Every case drives the real router over this binary's private database and parses the SSE body
//! frame by frame; every event's data is validated against `audit-log.schema.json`. The cases
//! share one publication sequence and one retained floor, so they run one at a time behind the
//! suite lock, and each compares what it received with the publication table.
//!
//! ## What makes this fail (non-vacuity)
//! `reset_reason` answering `None` where the cursor is below the retained floor turns
//! `audit_replay_cursor_below_the_floor_resets_history_unavailable_then_continues_live`,
//! `audit_replay_cursor_at_the_floor_replays_and_one_below_resets`,
//! `audit_replay_deleting_published_rows_while_connected_resets_to_the_tail` and
//! `audit_replay_backfill_floor_is_the_highest_missing_sequence` red.

mod audit_stream_support;
mod common;
mod contract_support;

use std::time::Duration;

use api::administration::services::audit_notifier::AuditSignal;
use axum::http::{HeaderValue, StatusCode};
use serde_json::{Value, json};
use tokio::sync::broadcast::error::TryRecvError;

use audit_stream_support::{
    AUDIT_SCHEMA, AuditHarness, SseEvent, case_tag, plant_row, plant_rows, publication_bounds,
    publications_after, publish_all, sequence_of, serialise_case, wait_signal,
};

const SUITE: &str = "audit_replay";

/// The bound on any event a case expects.
const BOUND: Duration = Duration::from_secs(5);

/// How long a case listens to prove nothing else arrives.
const QUIET: Duration = Duration::from_millis(500);

fn assert_ready(event: &SseEvent, resume_after: i64, retained_after: i64) {
    assert_eq!(event.event.as_deref(), Some("ready"), "{event:?}");
    assert_eq!(
        event.sequence(),
        resume_after,
        "ready's id is the start cursor"
    );
    let data = event.json();
    contract_support::assert_valid(AUDIT_SCHEMA, Some("AuditStreamReady"), &data);
    assert_eq!(
        data,
        json!({ "resume_after": resume_after, "retained_after": retained_after })
    );
}

fn assert_reset(event: &SseEvent, reason: &str, tail: i64, retained_after: i64) {
    assert_eq!(event.event.as_deref(), Some("reset"), "{event:?}");
    assert_eq!(event.sequence(), tail, "a reset's id is the tail");
    let data = event.json();
    contract_support::assert_valid(AUDIT_SCHEMA, Some("AuditStreamReset"), &data);
    assert_eq!(
        data,
        json!({ "reason": reason, "resume_after": tail, "retained_after": retained_after })
    );
}

/// An unnamed event whose id is `publication.0` and whose data is audit row `publication.1`.
fn assert_row(event: &SseEvent, publication: (i64, i64)) {
    assert!(
        event.is_row(),
        "an audit row is an unnamed event: {event:?}"
    );
    assert_eq!(
        event.sequence(),
        publication.0,
        "a row's id is its sequence"
    );
    let data = event.json();
    contract_support::assert_valid(AUDIT_SCHEMA, Some("AuditLogEntry"), &data);
    assert_eq!(data["id"], publication.1, "{data}");
}

fn received(events: &[SseEvent]) -> Vec<(i64, i64)> {
    events
        .iter()
        .map(|event| {
            (
                event.sequence(),
                event.json()["id"].as_i64().expect("row id"),
            )
        })
        .collect()
}

#[tokio::test]
async fn audit_replay_ready_carries_the_tail_without_last_event_id() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let tag = case_tag("ready");
    plant_rows(&harness.pool, &tag, 3).await;
    publish_all(&harness.pool).await;
    let (tail, floor) = publication_bounds(&harness.pool).await;
    assert!(tail >= 3, "the planted rows are published");

    let mut stream = harness.open_stream(None).await;
    let ready = stream.expect_event(BOUND, "ready opens the stream").await;
    assert_ready(&ready, tail, floor);
    let mut without_floor = ready.json();
    without_floor
        .as_object_mut()
        .expect("ready data is an object")
        .remove("retained_after");
    contract_support::assert_invalid(AUDIT_SCHEMA, Some("AuditStreamReady"), &without_floor);

    // Rows published before the open are history: the first row streamed is the next one.
    stream
        .expect_quiet(QUIET, "nothing after the tail yet")
        .await;
    let live = plant_row(&harness.pool, &tag, 4).await;
    let row = stream.expect_row(BOUND, "the live row").await;
    let expected = publications_after(&harness.pool, tail).await;
    assert_eq!(
        expected.len(),
        1,
        "one publication after the tail: {expected:?}"
    );
    assert_eq!(expected[0].1, live);
    assert_row(&row, expected[0]);
}

#[tokio::test]
async fn audit_replay_last_event_id_replays_exactly_the_later_rows_in_sequence_order() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    let tag = case_tag("replay");
    let delivered_before = plant_row(pool, &tag, 0).await;
    publish_all(pool).await;
    let cursor = sequence_of(pool, delivered_before)
        .await
        .expect("published");

    // The lower id commits last, so it is published after the higher id.
    let mut held = pool.begin().await.expect("begin");
    let lower = plant_row(&mut *held, &tag, 1).await;
    let higher = plant_row(pool, &tag, 2).await;
    publish_all(pool).await;
    held.commit().await.expect("commit the lower id");
    let later = plant_rows(pool, &tag, 3).await;
    publish_all(pool).await;
    let expected = publications_after(pool, cursor).await;
    let ids: Vec<i64> = expected.iter().map(|publication| publication.1).collect();
    assert_eq!(ids, [vec![higher, lower], later].concat());
    assert!(lower < higher);
    let (_, floor) = publication_bounds(pool).await;

    let mut stream = harness.open_stream(Some(cursor)).await;
    assert_ready(&stream.expect_event(BOUND, "ready").await, cursor, floor);
    let rows = stream
        .expect_rows(expected.len(), BOUND, "the rows after the cursor")
        .await;
    for (row, publication) in rows.iter().zip(&expected) {
        assert_row(row, *publication);
    }
    assert_eq!(
        received(&rows),
        expected,
        "exactly the later rows, in sequence order"
    );

    // Each streamed row is the list route's row, byte for byte in JSON.
    let (status, page) = harness.get_json("/api/v1/admin/audit-logs?limit=100").await;
    assert_eq!(status, StatusCode::OK, "{page}");
    contract_support::assert_valid(AUDIT_SCHEMA, None, &page);
    for row in &rows {
        let data = row.json();
        let listed = page["data"]
            .as_array()
            .expect("data")
            .iter()
            .find(|entry| entry["id"] == data["id"])
            .unwrap_or_else(|| panic!("row {} is on the first list page", data["id"]));
        assert_eq!(&data, listed);
    }

    // Nothing is replayed twice: the next event is the next row.
    let sentinel = plant_row(pool, &tag, 9).await;
    let next = stream.expect_row(BOUND, "the sentinel").await;
    let sentinel_sequence = sequence_of(pool, sentinel).await.expect("published");
    assert_row(&next, (sentinel_sequence, sentinel));
}

#[tokio::test]
async fn audit_replay_reconnect_mid_stream_has_no_gap_and_no_duplicate() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    let tag = case_tag("reconnect");
    publish_all(pool).await;
    let (start, floor) = publication_bounds(pool).await;

    let mut first = harness.open_stream(None).await;
    assert_ready(
        &first.expect_event(BOUND, "first ready").await,
        start,
        floor,
    );
    plant_rows(pool, &tag, 3).await;
    let before_drop = first.expect_rows(2, BOUND, "two of three rows").await;
    drop(first);

    let resume = before_drop[1].sequence();
    plant_rows(pool, &tag, 2).await;
    let mut second = harness.open_stream(Some(resume)).await;
    assert_ready(
        &second.expect_event(BOUND, "second ready").await,
        resume,
        floor,
    );
    let missed = publications_after(pool, resume).await;
    assert_eq!(
        missed.len(),
        3,
        "the undelivered row and the two offline rows"
    );
    let mut after_reconnect = second.expect_rows(3, BOUND, "the missed rows").await;
    plant_row(pool, &tag, 6).await;
    after_reconnect.push(second.expect_row(BOUND, "the live row").await);

    let mut delivered = received(&before_drop);
    delivered.extend(received(&after_reconnect));
    let published = publications_after(pool, start).await;
    assert_eq!(published.len(), 6);
    assert_eq!(delivered, published, "every row once, in sequence order");
    for (event, publication) in before_drop.iter().chain(&after_reconnect).zip(&published) {
        assert_row(event, *publication);
    }
    second
        .expect_quiet(QUIET, "no duplicate after the live row")
        .await;
}

#[tokio::test]
async fn audit_replay_slow_consumer_receives_a_700_row_backlog_in_order() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    let tag = case_tag("backlog");
    publish_all(pool).await;
    let (start, floor) = publication_bounds(pool).await;

    let mut stream = harness.open_stream(None).await;
    assert_ready(&stream.expect_event(BOUND, "ready").await, start, floor);
    let mut lag_witness = harness.notify.subscribe();
    let mut probe = harness.notify.subscribe();
    let planted = plant_rows(pool, &tag, 700).await;
    let last = *planted.last().expect("700 rows");
    wait_signal(
        &mut probe,
        AuditSignal::Row(last),
        Duration::from_secs(10),
        "the last of 700 notifications",
    )
    .await;
    assert!(
        matches!(lag_witness.try_recv(), Err(TryRecvError::Lagged(_))),
        "700 notifications overflow the notifier's buffer while nobody reads"
    );

    let rows = stream
        .expect_rows(700, Duration::from_secs(60), "the whole backlog")
        .await;
    let expected = publications_after(pool, start).await;
    assert_eq!(expected.len(), 700);
    let ids: Vec<i64> = expected.iter().map(|publication| publication.1).collect();
    assert_eq!(ids, planted, "one publication per planted row, in id order");
    assert_eq!(
        received(&rows),
        expected,
        "every row once, in sequence order"
    );
    let entry = contract_support::validator(AUDIT_SCHEMA, Some("AuditLogEntry"));
    for (row, pair) in rows.iter().zip(rows.iter().skip(1)) {
        assert_eq!(pair.sequence(), row.sequence() + 1, "no gap");
        assert!(
            entry.is_valid(&row.json()),
            "row data is an AuditLogEntry: {row:?}"
        );
    }

    let sentinel = plant_row(pool, &tag, 701).await;
    let next = stream.expect_row(BOUND, "the sentinel").await;
    assert_row(&next, (expected[699].0 + 1, sentinel));
}

#[tokio::test]
async fn audit_replay_cursor_below_the_floor_resets_history_unavailable_then_continues_live() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    let tag = case_tag("below-floor");
    let planted = plant_rows(pool, &tag, 3).await;
    publish_all(pool).await;
    let first = sequence_of(pool, planted[0]).await.expect("published");
    let removed = sequence_of(pool, planted[1]).await.expect("published");

    // Deleting the audit row cascades to its publication; the trigger raises the floor.
    sqlx::query("DELETE FROM audit_logs WHERE id = $1")
        .bind(planted[1])
        .execute(pool)
        .await
        .expect("delete a published row");
    let (tail, floor) = publication_bounds(pool).await;
    assert_eq!(floor, removed, "the floor is the deleted sequence");

    let mut stream = harness.open_stream(Some(first)).await;
    assert_ready(&stream.expect_event(BOUND, "ready").await, first, floor);
    let reset = stream.expect_event(BOUND, "the reset").await;
    assert_reset(&reset, "history_unavailable", tail, floor);
    let mut unknown_reason = reset.json();
    unknown_reason["reason"] = Value::from("gone");
    contract_support::assert_invalid(AUDIT_SCHEMA, Some("AuditStreamReset"), &unknown_reason);

    // The stream continues after the tail: the retained third row is not replayed.
    let live = plant_row(pool, &tag, 4).await;
    let row = stream.expect_row(BOUND, "the live row").await;
    assert_row(&row, (tail + 1, live));
    assert_eq!(sequence_of(pool, live).await, Some(tail + 1));
}

#[tokio::test]
async fn audit_replay_cursor_at_the_floor_replays_and_one_below_resets() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    let tag = case_tag("at-floor");
    let planted = plant_rows(pool, &tag, 3).await;
    publish_all(pool).await;
    let removed = sequence_of(pool, planted[0]).await.expect("published");
    sqlx::query("DELETE FROM audit_publications WHERE sequence = $1")
        .bind(removed)
        .execute(pool)
        .await
        .expect("delete a publication");
    let (tail, floor) = publication_bounds(pool).await;
    assert_eq!(floor, removed);

    let mut at_floor = harness.open_stream(Some(floor)).await;
    assert_ready(&at_floor.expect_event(BOUND, "ready").await, floor, floor);
    let rows = at_floor
        .expect_rows(2, BOUND, "the rows above the floor")
        .await;
    assert_eq!(received(&rows), publications_after(pool, floor).await);

    let mut below = harness.open_stream(Some(floor - 1)).await;
    assert_ready(&below.expect_event(BOUND, "ready").await, floor - 1, floor);
    let reset = below.expect_event(BOUND, "the reset").await;
    assert_reset(&reset, "history_unavailable", tail, floor);
    below
        .expect_quiet(QUIET, "nothing below the tail after a reset")
        .await;
}

#[tokio::test]
async fn audit_replay_cursor_ahead_of_the_tail_resets_cursor_ahead() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    let tag = case_tag("ahead");
    plant_row(pool, &tag, 0).await;
    publish_all(pool).await;
    let (tail, floor) = publication_bounds(pool).await;
    let ahead = tail + 1000;

    let mut stream = harness.open_stream(Some(ahead)).await;
    assert_ready(&stream.expect_event(BOUND, "ready").await, ahead, floor);
    let reset = stream.expect_event(BOUND, "the reset").await;
    assert_reset(&reset, "cursor_ahead", tail, floor);

    // A cursor at the tail is not ahead: no reset, the next row follows.
    let mut at_tail = harness.open_stream(Some(tail)).await;
    assert_ready(&at_tail.expect_event(BOUND, "ready").await, tail, floor);
    let live = plant_row(pool, &tag, 1).await;
    assert_row(
        &stream.expect_row(BOUND, "live after reset").await,
        (tail + 1, live),
    );
    assert_row(
        &at_tail.expect_row(BOUND, "live at tail").await,
        (tail + 1, live),
    );
}

#[tokio::test]
async fn audit_replay_deleting_published_rows_while_connected_resets_to_the_tail() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    let tag = case_tag("delete-live");
    publish_all(pool).await;
    let (start, floor) = publication_bounds(pool).await;
    let mut stream = harness.open_stream(None).await;
    assert_ready(&stream.expect_event(BOUND, "ready").await, start, floor);
    plant_rows(pool, &tag, 2).await;
    let delivered = stream.expect_rows(2, BOUND, "two live rows").await;

    // Deleting rows the client already holds leaves its cursor replayable: no reset.
    sqlx::query("DELETE FROM audit_publications WHERE sequence = $1")
        .bind(delivered[0].sequence())
        .execute(pool)
        .await
        .expect("delete a delivered publication");
    assert_eq!(publication_bounds(pool).await.1, delivered[0].sequence());
    let third = plant_row(pool, &tag, 3).await;
    let row = stream
        .expect_row(BOUND, "the next live row, not a reset")
        .await;
    assert_row(&row, (delivered[1].sequence() + 1, third));

    // While the client has not yet read them, two rows are published and the later is deleted:
    // the floor passes the client's cursor.
    let unread = plant_rows(pool, &tag, 2).await;
    publish_all(pool).await;
    sqlx::query("DELETE FROM audit_logs WHERE id = $1")
        .bind(unread[1])
        .execute(pool)
        .await
        .expect("delete an unread row");
    let (tail, floor) = publication_bounds(pool).await;
    assert_eq!(floor, tail, "the deleted row was the tail");
    assert!(floor > row.sequence());

    let reset = stream.expect_event(BOUND, "the reset").await;
    assert_reset(&reset, "history_unavailable", tail, floor);
    let live = plant_row(pool, &tag, 6).await;
    assert_row(
        &stream.expect_row(BOUND, "live after reset").await,
        (tail + 1, live),
    );
}

#[tokio::test]
async fn audit_replay_malformed_last_event_id_answers_400() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let malformed = [
        HeaderValue::from_static("abc"),
        HeaderValue::from_static("-1"),
        HeaderValue::from_static("1.5"),
        HeaderValue::from_static(""),
        HeaderValue::from_static("12abc"),
        HeaderValue::from_static("9223372036854775808"),
        HeaderValue::from_bytes(b"\xff").expect("opaque header bytes"),
    ];
    for value in malformed {
        let shown = format!("{value:?}");
        let response = harness.request_stream(Some(value)).await;
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Last-Event-ID {shown}"
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read the refusal");
        let body: Value = serde_json::from_slice(&bytes).expect("a JSON refusal");
        assert!(body["error"].is_string(), "the error envelope: {body}");
    }
    let mut zero = harness.open_stream(Some(0)).await;
    let (_, floor) = publication_bounds(&harness.pool).await;
    assert_ready(
        &zero.expect_event(BOUND, "cursor 0 is well formed").await,
        0,
        floor,
    );
}

/// Migration 0057's backfill statement, verbatim.
fn backfill_statement() -> &'static str {
    let migration = include_str!("../migrations/0057_audit_publication_retained_floor.sql");
    let start = migration
        .find("UPDATE public.audit_publication_state AS state")
        .expect("0057 backfills the floor");
    let length = migration[start..].find(';').expect("the backfill ends") + 1;
    &migration[start..start + length]
}

/// The highest sequence at or below the tail that is missing, or 0: the backfill's definition,
/// computed here from the publication table.
async fn highest_missing_sequence(pool: &sqlx::PgPool) -> i64 {
    let (tail, _) = publication_bounds(pool).await;
    let present: std::collections::HashSet<i64> =
        sqlx::query_scalar("SELECT sequence FROM audit_publications WHERE sequence <= $1")
            .bind(tail)
            .fetch_all(pool)
            .await
            .expect("read present sequences")
            .into_iter()
            .collect();
    (1..=tail)
        .rev()
        .find(|sequence| !present.contains(sequence))
        .unwrap_or(0)
}

/// Forgets the floor, as a database that predates 0057 does, and runs 0057's backfill.
async fn backfill_floor(pool: &sqlx::PgPool) {
    sqlx::query("UPDATE audit_publication_state SET retained_after_sequence = 0 WHERE singleton")
        .execute(pool)
        .await
        .expect("forget the floor");
    sqlx::raw_sql(backfill_statement())
        .execute(pool)
        .await
        .expect("run the 0057 backfill");
}

#[tokio::test]
async fn audit_replay_backfill_floor_is_the_highest_missing_sequence() {
    let _case = serialise_case().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    let tag = case_tag("backfill");
    let planted = plant_rows(pool, &tag, 2).await;
    publish_all(pool).await;
    let gap = sequence_of(pool, planted[0]).await.expect("published");
    let present = sequence_of(pool, planted[1]).await.expect("published");

    // A gap below a present sequence: the backfilled floor is the gap.
    sqlx::query("DELETE FROM audit_publications WHERE sequence = $1")
        .bind(gap)
        .execute(pool)
        .await
        .expect("open a gap");
    backfill_floor(pool).await;
    assert_eq!(highest_missing_sequence(pool).await, gap);
    let (tail, floor) = publication_bounds(pool).await;
    assert_eq!((tail, floor), (present, gap));
    let mut at_tail = harness.open_stream(None).await;
    assert_ready(&at_tail.expect_event(BOUND, "ready").await, tail, gap);
    let mut below = harness.open_stream(Some(gap - 1)).await;
    assert_ready(&below.expect_event(BOUND, "ready").await, gap - 1, gap);
    assert_reset(
        &below.expect_event(BOUND, "reset").await,
        "history_unavailable",
        tail,
        gap,
    );
    let mut at_gap = harness.open_stream(Some(gap)).await;
    assert_ready(&at_gap.expect_event(BOUND, "ready").await, gap, gap);
    assert_row(
        &at_gap.expect_row(BOUND, "the present row").await,
        (present, planted[1]),
    );

    // The tail itself missing: the backfilled floor is the tail.
    sqlx::query("DELETE FROM audit_publications WHERE sequence = $1")
        .bind(present)
        .execute(pool)
        .await
        .expect("remove the tail");
    backfill_floor(pool).await;
    assert_eq!(highest_missing_sequence(pool).await, present);
    assert_eq!(publication_bounds(pool).await, (present, present));
    let mut emptied = harness.open_stream(None).await;
    assert_ready(
        &emptied.expect_event(BOUND, "ready").await,
        present,
        present,
    );
    let mut behind = harness.open_stream(Some(gap)).await;
    assert_ready(&behind.expect_event(BOUND, "ready").await, gap, present);
    assert_reset(
        &behind.expect_event(BOUND, "reset").await,
        "history_unavailable",
        present,
        present,
    );
}
