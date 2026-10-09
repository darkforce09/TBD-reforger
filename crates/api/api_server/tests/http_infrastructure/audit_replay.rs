//! Replay and reset of `GET /api/v1/admin/audit-logs/stream`: `Last-Event-ID` replays exactly the
//! later rows in publication order, and a cursor below the retained floor of migration 0057 resets
//! with `history_unavailable` and then continues live.
//!
//! Every case drives the real router over the binary's isolated `audit_stream` database and parses
//! the SSE body frame by frame; every event's data is validated against `audit-log.schema.json`.
//! The cases share one publication sequence and one retained floor, so they run one at a time
//! behind the suite lock, and each compares what it received with the publication table.

use crate::{audit_stream_support, contract_support};

use std::time::Duration;

use axum::http::StatusCode;
use serde_json::{Value, json};

use audit_stream_support::{
    AUDIT_SCHEMA, AuditHarness, SseEvent, case_tag, plant_row, plant_rows, publication_bounds,
    publications_after, publish_all, sequence_of, serialise_case,
};

const SUITE: &str = "audit_replay";

/// The bound on any event a case expects.
const BOUND: Duration = Duration::from_secs(5);

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
