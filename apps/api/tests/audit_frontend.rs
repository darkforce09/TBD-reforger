//! The audit logs page protocol against the real router: connect the stream, wait for `ready`,
//! load the keyset history, merge live and listed rows by audit id, and reload on `reset`.
//!
//! **Role:** the `administration_audit_frontend` requirement: the page the protocol builds holds
//! exactly the database's rows, whatever interleaving of history pages, live rows, concurrent
//! writers and resets it meets.
//! **Position:** drives `GET /api/v1/admin/audit-logs/stream` and `GET /api/v1/admin/audit-logs`
//! through [`audit_frontend_support::AuditLogsPage`]; rows are written through
//! `administration::services::required_audit` and the audited warning route.
//! **Signals & state:** every case holds the suite's sequence lock (see
//! `audit_frontend_support`), owns a marker its rows carry, and counts only marked rows.
//! **Invariants:** the merged view equals the database set for the case's marker; one audit id
//! is one JSON value on both routes; a system row carries the time of the transaction that
//! appended it on both routes; a reset is followed by a history reload.

mod audit_frontend_support;
mod common;
mod contract_support;

use std::collections::BTreeSet;
use std::time::Duration;

use api::administration::models::audit_log::AuditSeverity;
use chrono::{DateTime, Datelike, Utc};
use serde_json::json;

use audit_frontend_support::{AuditConsoleFixture, AuditLogsPage, SCHEMA, StreamStep};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn audit_frontend_merged_view_equals_the_database_rows_with_a_non_empty_overlap() {
    let fixture = AuditConsoleFixture::boot().await;
    let mut before_open = BTreeSet::new();
    for n in 0..8 {
        before_open.insert(fixture.write_row(&format!("before-open {n}")).await);
    }

    let mut page = AuditLogsPage::connect(&fixture, 4).await;
    // Committed after ready and before the history load: listed and streamed both.
    let mut overlap = BTreeSet::new();
    for n in 0..5 {
        overlap.insert(fixture.write_row(&format!("overlap {n}")).await);
    }
    let writer = fixture.spawn_writer("concurrent", 12, Duration::from_millis(15));
    page.load_history().await;
    let concurrent: BTreeSet<i64> = writer.await.expect("writer task").into_iter().collect();
    let tail = fixture.publish_everything().await;
    page.catch_up(tail).await;

    let database = fixture.database_ids().await;
    assert_eq!(database.len(), 25, "8 + 5 + 12 rows carry the marker");
    assert_eq!(
        page.view.ids(),
        database,
        "the merged view is the database set"
    );
    assert!(
        overlap.is_subset(&page.view.overlap()),
        "rows committed between ready and the history load arrive both ways: overlap {:?}",
        page.view.overlap()
    );
    assert!(!page.view.overlap().is_empty());
    assert!(
        before_open.is_disjoint(&page.view.from_stream),
        "the stream opens at the tail, after the rows written before it"
    );
    assert!(
        concurrent.is_subset(&page.view.from_stream),
        "every row committed after ready arrives live"
    );
    assert!(
        page.resets.is_empty(),
        "nothing forces a reset: {:?}",
        page.resets
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn audit_frontend_stream_rows_and_list_rows_are_the_same_audit_log_entry() {
    let fixture = AuditConsoleFixture::boot().await;
    let member = fixture.seed_member().await;
    let mut page = AuditLogsPage::connect(&fixture, 20).await;

    let actor_row = fixture.write_row("actor").await;
    let critical_row = fixture
        .write_row_with_severity(AuditSeverity::Crit, "critical")
        .await;
    let system_row = fixture.write_system_row("system").await;
    let warning_row = fixture.warn_over_http(&member, "warning").await;
    page.load_history().await;
    let tail = fixture.publish_everything().await;
    page.catch_up(tail).await;

    let rows = [actor_row, critical_row, system_row, warning_row];
    assert_eq!(page.view.ids(), BTreeSet::from(rows));
    for id in rows {
        let listed = page.view.history_row(id);
        let streamed = page.view.stream_row(id);
        contract_support::assert_valid(SCHEMA, Some("AuditLogEntry"), listed);
        contract_support::assert_valid(SCHEMA, Some("AuditLogEntry"), streamed);
        let listed_keys: BTreeSet<&String> = listed.as_object().unwrap().keys().collect();
        let streamed_keys: BTreeSet<&String> = streamed.as_object().unwrap().keys().collect();
        assert_eq!(
            listed_keys, streamed_keys,
            "audit id {id} has the same keys both ways"
        );
        assert_eq!(
            listed, streamed,
            "audit id {id} is the same entry both ways"
        );
    }

    let severities: Vec<_> = rows
        .iter()
        .map(|id| page.view.history_row(*id)["severity"].clone())
        .collect();
    assert_eq!(
        severities,
        [json!("info"), json!("crit"), json!("info"), json!("warn")]
    );
    let actor = page.view.stream_row(actor_row);
    assert_eq!(actor["actor_id"], json!(fixture.admin_id));
    assert_eq!(actor["target_type"], json!("user"));
    let system = page.view.stream_row(system_row);
    assert!(
        system.get("actor_id").is_none(),
        "a system row omits actor_id: {system}"
    );
    assert_eq!(system["actor_name"], json!("system"));
    let warning = page.view.stream_row(warning_row);
    assert_eq!(warning["target_id"], json!(member));

    // The publication sequence travels as the SSE id, never as a row field.
    let mut with_sequence = actor.clone();
    with_sequence["sequence"] = json!(1);
    contract_support::assert_invalid(SCHEMA, Some("AuditLogEntry"), &with_sequence);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn audit_frontend_system_rows_carry_their_transaction_time_on_the_list_and_the_stream() {
    let fixture = AuditConsoleFixture::boot().await;
    let mut page = AuditLogsPage::connect(&fixture, 5).await;

    let (system_row, transaction_time) = fixture.write_timed_system_row("timed system").await;
    page.load_history().await;
    let tail = fixture.publish_everything().await;
    page.catch_up(tail).await;

    for (way, row) in [
        ("listed", page.view.history_row(system_row)),
        ("streamed", page.view.stream_row(system_row)),
    ] {
        let served = row["created_at"]
            .as_str()
            .unwrap_or_else(|| panic!("{way}: created_at is a string: {row}"));
        let served = DateTime::parse_from_rfc3339(served)
            .unwrap_or_else(|error| panic!("{way}: created_at is RFC 3339 ({error}): {row}"))
            .with_timezone(&Utc);
        assert_ne!(
            served.year(),
            1,
            "{way}: not the 0001-01-01 fallback: {row}"
        );
        assert_eq!(
            served, transaction_time,
            "{way}: the system row carries its transaction time: {row}"
        );
    }
    let stored: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT created_at FROM audit_logs WHERE id = $1")
            .bind(system_row)
            .fetch_one(&fixture.pool)
            .await
            .expect("read the stored system row");
    assert_eq!(
        stored,
        Some(transaction_time),
        "the system row stores the time of the transaction that appended it"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn audit_frontend_reset_from_deleted_published_rows_reloads_a_consistent_history() {
    let fixture = AuditConsoleFixture::boot().await;
    let mut page = AuditLogsPage::connect(&fixture, 3).await;
    let opened_at = page.cursor;

    let mut shown = Vec::new();
    for n in 0..4 {
        shown.push(fixture.write_row(&format!("shown {n}")).await);
    }
    page.load_history().await;
    // Committed after the history load and published before the stream reads them.
    let mut unseen = Vec::new();
    for n in 0..3 {
        unseen.push(fixture.write_row(&format!("unseen {n}")).await);
    }
    let tail = fixture.publish_everything().await;

    let deleted = [shown[0], shown[1]];
    assert!(page.view.ids().is_superset(&BTreeSet::from(deleted)));
    let highest_removed = fixture.delete_published_rows(&deleted).await;
    assert!(
        highest_removed > opened_at,
        "the removed history lies after the stream's cursor"
    );
    assert_eq!(fixture.retained_floor().await, highest_removed);

    let StreamStep::Reset(reset) = page.step().await else {
        panic!(
            "the stream resets before any row: {:?}",
            page.view.from_stream
        );
    };
    assert_eq!(reset["reason"], json!("history_unavailable"));
    assert_eq!(reset["resume_after"], json!(tail));
    assert_eq!(reset["retained_after"], json!(highest_removed));
    assert_eq!(page.cursor, tail, "the stream continues after the tail");
    assert_eq!(
        page.view.ids(),
        fixture.database_ids().await,
        "the reloaded history is the database set"
    );
    for id in deleted {
        assert!(
            !page.view.ids().contains(&id),
            "deleted audit id {id} is no longer shown"
        );
    }

    let mut after_reset = Vec::new();
    for n in 0..3 {
        after_reset.push(fixture.write_row(&format!("after-reset {n}")).await);
    }
    let tail = fixture.publish_everything().await;
    page.catch_up(tail).await;

    assert_eq!(page.view.ids(), fixture.database_ids().await);
    assert_eq!(page.view.ids().len(), 2 + 3 + 3);
    for id in &unseen {
        assert!(
            !page.view.from_stream.contains(id),
            "audit id {id} lies before the reset's tail and reaches the page only by the reload"
        );
    }
    for id in &after_reset {
        assert!(
            page.view.from_stream.contains(id),
            "audit id {id} arrives live"
        );
    }
    assert_eq!(page.resets.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn audit_frontend_keyset_pages_and_the_live_stream_lose_nothing_under_concurrent_writes() {
    let fixture = AuditConsoleFixture::boot().await;
    for n in 0..23 {
        fixture.write_row(&format!("history {n}")).await;
    }
    let mut page = AuditLogsPage::connect(&fixture, 5).await;

    // Allocates its id before the concurrent writers and commits after the history is paged,
    // below rows the first page already listed.
    let (held, held_id) = fixture.begin_held_row("held").await;
    let writers: Vec<_> = (0..3u64)
        .map(|writer| {
            fixture.spawn_writer(
                &format!("writer-{writer}"),
                10,
                Duration::from_millis(5 + 4 * writer),
            )
        })
        .collect();

    let mut before = None;
    loop {
        let next = page.load_history_page(before).await;
        page.pump(Duration::from_millis(40)).await;
        match next {
            Some(cursor) => before = Some(cursor),
            None => break,
        }
    }
    let mut written = BTreeSet::new();
    for writer in writers {
        written.extend(writer.await.expect("writer task"));
    }
    held.commit().await.expect("commit the held audit row");
    let tail = fixture.publish_everything().await;
    page.catch_up(tail).await;

    let database = fixture.database_ids().await;
    assert_eq!(database.len(), 23 + 30 + 1);
    assert_eq!(
        page.view.ids(),
        database,
        "history pages plus the stream hold every row"
    );
    assert!(written.iter().all(|id| *id > held_id));
    assert!(
        !page.view.from_history.contains(&held_id) && page.view.from_stream.contains(&held_id),
        "the late-committing lower id {held_id} arrives live"
    );
    assert!(
        page.history_pages_loaded >= 5,
        "the history is read in keyset pages of 5: {}",
        page.history_pages_loaded
    );
    assert!(
        page.resets.is_empty(),
        "nothing forces a reset: {:?}",
        page.resets
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn audit_frontend_cursor_ahead_reset_at_open_reloads_the_history() {
    let fixture = AuditConsoleFixture::boot().await;
    for n in 0..4 {
        fixture.write_row(&format!("before-open {n}")).await;
    }
    let tail = fixture.publish_everything().await;
    let stale = tail + 1000;

    let mut page = AuditLogsPage::connect_after(&fixture, 3, Some(stale)).await;
    assert_eq!(
        page.ready["resume_after"],
        json!(stale),
        "ready echoes the requested cursor"
    );
    page.load_history().await;
    let StreamStep::Reset(reset) = page.step().await else {
        panic!("a cursor beyond the tail resets right after ready");
    };
    assert_eq!(reset["reason"], json!("cursor_ahead"));
    assert_eq!(reset["resume_after"], json!(tail));

    let live = fixture.write_row("after-reset").await;
    let tail = fixture.publish_everything().await;
    page.catch_up(tail).await;
    assert_eq!(page.view.ids(), fixture.database_ids().await);
    assert!(
        page.view.from_stream.contains(&live),
        "rows after the reset arrive live"
    );
    assert_eq!(page.view.ids().len(), 5);
}
