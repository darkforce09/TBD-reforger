//! Failure injection in audit publication and delivery.
//!
//! **Role:** proves that a publication failed before its commit is retried by the publication
//! worker with every row published once in a contiguous sequence; that a client reconnecting with
//! its cursor while every delivery read fails receives, once reads succeed, exactly the rows after
//! that cursor; and that a listener whose socket stays open but carries no notification (a
//! half-open connection: `is_listening()` stays `true`) does not stop delivery, because the
//! stream's timer poll delivers the row.
//! **Position:** its own test binary over `publish_audit_batch` (failpoint
//! `AuditPublicationBeforeCommit`) driven by `start_audit_publication`, over
//! `GET /api/v1/admin/audit-logs/stream` with `Last-Event-ID` and `audit_delivery_stream`
//! (failpoint `AuditDeliveryRead`), using the harness of `tests/audit_stream_support`. The
//! half-open listener runs over a local TCP relay whose server-to-client half is silenced while
//! both sockets stay open.
//! **Signals & state:** the process-global failpoint registry and the one publication sequence of
//! this binary's private database, both serialised by the suite lock every case holds for its
//! whole body; nothing but the code under test publishes while a case runs.
//! **Invariants:** every wait is bounded; the publication table read after the fact is the oracle
//! each delivered sequence is compared with.

mod audit_stream_support;
mod common;
mod failpoint_and_race_support;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use api::administration::services::audit_delivery::{AuditStreamItem, audit_delivery_stream};
use api::administration::services::audit_notifier::{AuditNotify, AuditSignal};
use api::background_workers::audit_publication_worker::start_audit_publication;
use audit_stream_support::{
    AuditHarness, SseEvent, SseReader, case_tag, drive, next_item, plant_row, plant_rows,
    publication_bounds, publications_after, publish_all, sequence_of, wait_listening,
    wait_published, wait_signal,
};
use failpoint_and_race_support::{
    ArmGuard, AuditEvidence, Failpoint, FailpointArming, RowCounts, audit_evidence,
    check_publication_sequence, lock_suite,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::error::TryRecvError;
use tokio::task::{JoinHandle, JoinSet};

const SUITE: &str = "failure_injection_audit";

/// The replay timer of the service-level stream.
const TICK: Duration = Duration::from_millis(200);

/// The bound on anything a case waits for; the HTTP stream polls every two seconds.
const BOUND: Duration = Duration::from_secs(10);

/// How long a case listens for an event that must not come.
const QUIET: Duration = Duration::from_millis(600);

/// Keeps reading `reader` (an SSE body advances only while it is read) until the armed point has
/// been reached `minimum` times, failing the case on any event meanwhile.
async fn read_nothing_until_arrivals(
    reader: &mut SseReader,
    guard: &ArmGuard<'_>,
    minimum: usize,
    why: &str,
) {
    let deadline = Instant::now() + BOUND;
    while guard.arrivals() < minimum {
        assert!(
            Instant::now() < deadline,
            "{} of {minimum} arrivals within {BOUND:?}: {why}",
            guard.arrivals()
        );
        reader
            .expect_quiet(
                Duration::from_millis(50),
                "no delivery while every read fails",
            )
            .await;
    }
}

/// Bounded wait until the armed point has been reached `minimum` times.
async fn wait_for_arrivals(guard: &ArmGuard<'_>, minimum: usize, why: &str) {
    let deadline = Instant::now() + BOUND;
    while guard.arrivals() < minimum {
        assert!(
            Instant::now() < deadline,
            "{} of {minimum} arrivals within {BOUND:?}: {why}",
            guard.arrivals()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn failure_injection_audit_publication_before_commit_is_retried_by_the_worker_without_loss_or_duplicate()
 {
    let suite = lock_suite().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    publish_all(pool).await;
    let (tail, _) = publication_bounds(pool).await;
    let tag = case_tag("publication-retry");
    let ids = plant_rows(pool, &tag, 3).await;
    let unpublished = RowCounts::capture(pool, &["audit_publications"]).await;

    let guard = suite.fail(Failpoint::AuditPublicationBeforeCommit);
    let worker = start_audit_publication(pool.clone());
    wait_for_arrivals(&guard, 2, "the worker retries the failed publication").await;
    unpublished.check_unchanged(pool).await.unwrap();
    assert_eq!(
        audit_evidence(pool, &tag, &tag).await,
        AuditEvidence {
            rows: 3,
            pending: 3,
            published: 0
        },
        "a failed publication keeps every row pending"
    );
    assert_eq!(
        publication_bounds(pool).await.0,
        tail,
        "no sequence is consumed"
    );
    drop(guard);

    let mut sequences = Vec::with_capacity(ids.len());
    for &id in &ids {
        sequences.push(wait_published(pool, id, BOUND).await);
    }
    worker.abort();
    let _ = worker.await;
    assert_eq!(sequences, [tail + 1, tail + 2, tail + 3]);
    let expected: Vec<(i64, i64)> = sequences.iter().copied().zip(ids.iter().copied()).collect();
    assert_eq!(publications_after(pool, tail).await, expected);
    assert_eq!(
        audit_evidence(pool, &tag, &tag).await,
        AuditEvidence {
            rows: 3,
            pending: 0,
            published: 3
        }
    );
    check_publication_sequence(pool).await.unwrap();
}

#[tokio::test]
async fn failure_injection_audit_delivery_read_failure_during_reconnect_resumes_from_the_cursor() {
    let suite = lock_suite().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    publish_all(pool).await;
    let tag = case_tag("reconnect-read");
    let ids = plant_rows(pool, &tag, 3).await;
    publish_all(pool).await;
    let mut sequences = Vec::with_capacity(ids.len());
    for &id in &ids {
        sequences.push(sequence_of(pool, id).await.expect("published"));
    }
    // The client received the first row, then lost its connection.
    let cursor = sequences[0];

    let guard = suite.fail(Failpoint::AuditDeliveryRead);
    let mut reader = harness.open_stream(Some(cursor)).await;
    let ready = reader.expect_event(BOUND, "ready").await;
    assert_eq!(ready.event.as_deref(), Some("ready"), "{ready:?}");
    assert_eq!(
        ready.sequence(),
        cursor,
        "the stream resumes after the cursor"
    );
    // A row published during the outage wakes the stream, whose read fails again.
    let late = plant_row(pool, &tag, 3).await;
    publish_all(pool).await;
    let late_sequence = sequence_of(pool, late).await.expect("published");
    read_nothing_until_arrivals(&mut reader, &guard, 2, "every wake retries the failed read").await;
    reader
        .expect_quiet(QUIET, "no delivery while every read fails")
        .await;
    drop(guard);

    let rows = reader
        .expect_rows(3, BOUND, "the rows after the cursor once reads succeed")
        .await;
    let delivered: Vec<(i64, i64)> = rows
        .iter()
        .map(|row: &SseEvent| (row.sequence(), row.json()["id"].as_i64().unwrap()))
        .collect();
    assert_eq!(
        delivered,
        [
            (sequences[1], ids[1]),
            (sequences[2], ids[2]),
            (late_sequence, late)
        ]
    );
    reader
        .expect_quiet(QUIET, "nothing is delivered twice")
        .await;
    check_publication_sequence(pool).await.unwrap();
}

/// A TCP relay between one pool and PostgreSQL whose server-to-client half can go silent while
/// both sockets stay open, which is how a half-open connection looks to the client.
struct SilenceableRelay {
    port: u16,
    silenced: Arc<AtomicBool>,
    relay: JoinHandle<()>,
}

impl SilenceableRelay {
    async fn start(upstream_host: String, upstream_port: u16) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind the relay");
        let port = listener.local_addr().expect("the relay address").port();
        let silenced = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&silenced);
        let relay = tokio::spawn(async move {
            // Dropping the set when the relay is aborted closes every relayed connection.
            let mut connections = JoinSet::new();
            while let Ok((client, _)) = listener.accept().await {
                let upstream = TcpStream::connect((upstream_host.as_str(), upstream_port)).await;
                if let Ok(server) = upstream {
                    connections.spawn(relay_connection(client, server, Arc::clone(&flag)));
                }
            }
        });
        Self {
            port,
            silenced,
            relay,
        }
    }

    /// From now on every byte PostgreSQL sends is dropped; the client's bytes still pass.
    fn silence_server_to_client(&self) {
        self.silenced.store(true, Ordering::Release);
    }
}

impl Drop for SilenceableRelay {
    fn drop(&mut self) {
        self.relay.abort();
    }
}

async fn relay_connection(client: TcpStream, server: TcpStream, silenced: Arc<AtomicBool>) {
    let (mut from_client, mut to_client) = client.into_split();
    let (mut from_server, mut to_server) = server.into_split();
    let upstream = tokio::io::copy(&mut from_client, &mut to_server);
    let downstream = async {
        let mut buffer = vec![0_u8; 16 * 1024];
        loop {
            let read = match from_server.read(&mut buffer).await {
                Ok(0) | Err(_) => return,
                Ok(read) => read,
            };
            if !silenced.load(Ordering::Acquire)
                && to_client.write_all(&buffer[..read]).await.is_err()
            {
                return;
            }
        }
    };
    tokio::select! {
        _ = upstream => {},
        () = downstream => {},
    }
}

async fn backend_alive(pool: &sqlx::PgPool, pid: u32) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE pid = $1)")
        .bind(i32::try_from(pid).expect("a backend pid fits an int4"))
        .fetch_one(pool)
        .await
        .expect("read pg_stat_activity")
}

#[tokio::test]
async fn failure_injection_audit_half_open_listener_stays_listening_and_the_timer_poll_still_delivers()
 {
    let _suite = lock_suite().await;
    let harness = AuditHarness::boot(SUITE).await;
    let pool = &harness.pool;
    publish_all(pool).await;

    let url = common::require_test_database_url()
        .expect("the audit failure-injection suite requires PostgreSQL");
    let direct: PgConnectOptions = url.parse().expect("parse the test database URL");
    assert!(
        !direct.get_host().starts_with('/'),
        "the relay needs a TCP database address, got {}",
        direct.get_host()
    );
    let relay = SilenceableRelay::start(direct.get_host().to_owned(), direct.get_port()).await;
    let relayed = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(direct.host("127.0.0.1").port(relay.port))
        .await
        .expect("connect through the relay");
    let notify = AuditNotify::for_pool(&relayed);
    wait_listening(&notify).await;
    let pid = notify.backend_pid().expect("the relayed listener is up");

    // While the relay passes both halves, the relayed listener announces every commit.
    let tag = case_tag("half-open");
    let mut probe = notify.subscribe();
    let announced = plant_row(pool, &tag, 0).await;
    wait_signal(
        &mut probe,
        AuditSignal::Row(announced),
        BOUND,
        "the relayed listener announces the insert",
    )
    .await;
    publish_all(pool).await;
    let announced_sequence = sequence_of(pool, announced).await.expect("published");
    wait_signal(
        &mut probe,
        AuditSignal::Row(announced_sequence),
        BOUND,
        "the relayed listener announces the publication",
    )
    .await;

    let items = audit_delivery_stream(pool.clone(), notify.clone(), TICK, None)
        .await
        .expect("open the delivery stream");
    let mut items = drive(items);
    let tail = match next_item(&mut items, BOUND).await {
        Some(AuditStreamItem::Ready(ready)) => ready.resume_after,
        other => panic!("expected ready, got {other:?}"),
    };
    assert_eq!(tail, announced_sequence);

    relay.silence_server_to_client();
    let mut silent = notify.subscribe();
    let row = plant_row(pool, &tag, 1).await;
    match next_item(&mut items, BOUND).await {
        Some(AuditStreamItem::Delivery(delivery)) => {
            assert_eq!(delivery.row.id, row);
            assert_eq!(Some(delivery.sequence), sequence_of(pool, row).await);
            assert!(delivery.sequence > tail);
        }
        other => panic!("expected the delivery of {row}, got {other:?}"),
    }
    assert!(
        matches!(silent.try_recv(), Err(TryRecvError::Empty)),
        "no notification reached the listener: the timer poll delivered the row"
    );
    assert!(
        notify.is_listening(),
        "the half-open listener still reports listening"
    );
    assert_eq!(notify.backend_pid(), Some(pid));
    assert!(
        backend_alive(pool, pid).await,
        "the listener's backend is still connected"
    );
    assert!(
        next_item(&mut items, QUIET).await.is_none(),
        "the row is delivered once"
    );
    check_publication_sequence(pool).await.unwrap();
}
