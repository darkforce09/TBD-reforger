//! Shutdown of the open event streams: once the process begins shutting down, the body of every
//! open stream (`GET /api/v1/admin/audit-logs/stream` and the server status stream) ends promptly
//! with no further event, the graceful drain completes and the `api-server` process exits, and a
//! client that reconnects with the last id it received gets every audit row written after that id
//! exactly once.
//!
//! The case runs the real `api-server` binary over this binary's private database for the SIGTERM
//! step, reconnects through the router in this process, and then begins this process's own shutdown
//! signal. A begun process shutdown ends every event stream opened afterwards in the same
//! process, so this binary holds exactly one case.
//!
//! ## What makes this fail (non-vacuity)
//! An `authorize_event_stream` that does not race the process shutdown, or a `bin/api_server.rs` that
//! does not begin it on SIGTERM, leaves the streams open after SIGTERM: the audit body never ends
//! and the case fails at "the audit stream on the api-server process ends after SIGTERM".

mod audit_shutdown_support;
mod common;
mod contract_support;

use std::time::{Duration, Instant};

use api_configuration::configuration::Config;
use api_configuration::process_lifecycle::process_shutdown;
use api_server::router::router;
use serde_json::json;
use uuid::Uuid;

use audit_shutdown_support::{
    AUDIT_STREAM_PATH, ApiProcess, SseEvent, http_client, open_process_stream, open_router_stream,
    plant_rows, publications_after, publish_all, register_silent_server, retained_floor,
    server_status_stream_path, wait_published,
};

const SUITE: &str = "audit_replay_shutdown";

/// The contract every stream event's data is validated against.
const AUDIT_SCHEMA: &str = "audit-log.schema.json";

/// The signing secret shared by this process and the `api-server` process it starts.
const JWT_SECRET: &str = "audit-replay-shutdown-secret";

/// The bound on any event a step expects; covers the stream's two-second fallback poll.
const EVENT_BOUND: Duration = Duration::from_secs(10);

/// The bound on a stream's body ending once shutdown begins.
const END_BOUND: Duration = Duration::from_secs(5);

/// The bound on the `api-server` process exiting after SIGTERM.
const EXIT_BOUND: Duration = Duration::from_secs(10);

/// How long a step listens to prove nothing else arrives.
const QUIET: Duration = Duration::from_millis(500);

/// Asserts the `ready` event opening a stream that resumes after `resume_after`.
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

/// `(sequence, audit id)` of every event, each validated as an audit row.
fn received(events: &[SseEvent]) -> Vec<(i64, i64)> {
    events
        .iter()
        .map(|event| {
            contract_support::assert_valid(AUDIT_SCHEMA, Some("AuditLogEntry"), &event.json());
            event.publication()
        })
        .collect()
}

#[tokio::test]
async fn audit_replay_shutdown_ends_open_streams_so_clients_reconnect_and_replay() {
    let url = common::require_test_database_url()
        .expect("the audit replay shutdown suite requires its PostgreSQL database");
    let pool = api_database::connect(&url)
        .await
        .expect("connect test database");
    api_database::migrate(&pool)
        .await
        .expect("migrate test database");
    let state = api_server::composition::application_state(
        pool.clone(),
        Config::for_tests(url.clone(), JWT_SECRET),
    );
    let admin = format!("{SUITE}-admin-{}", Uuid::new_v4());
    let token = common::access_token(&state, SUITE, &admin, "admin", true).await;
    let tag = format!("{SUITE}.{}", Uuid::new_v4());

    // 1. The api-server binary serves both streams; SIGTERM closes them and the process exits.
    let mut api = ApiProcess::start(&url, JWT_SECRET, state.cfg.discord_guild_id.as_str()).await;
    let client = http_client(None);
    let mut audit = open_process_stream(&api, &client, &token, AUDIT_STREAM_PATH, None).await;
    let ready = audit
        .expect_event(
            EVENT_BOUND,
            "ready opens the audit stream on the api-server process",
        )
        .await;
    let opened_after = ready.sequence();
    assert_ready(&ready, opened_after, retained_floor(&pool).await);
    let status_path = server_status_stream_path(register_silent_server(&pool, SUITE).await);
    let mut status = open_process_stream(&api, &client, &token, &status_path, None).await;

    let served = plant_rows(&pool, &tag, 3).await;
    for audit_id in &served {
        wait_published(&pool, *audit_id, EVENT_BOUND).await;
    }
    let published = publications_after(&pool, opened_after).await;
    let live = audit
        .expect_rows(
            published.len(),
            EVENT_BOUND,
            "the rows written while the api-server process serves",
        )
        .await;
    assert_eq!(
        received(&live),
        published,
        "the live rows, in sequence order"
    );

    let signalled = api.terminate();
    let before_end = audit
        .read_to_end(
            END_BOUND,
            "the audit stream on the api-server process ends after SIGTERM",
        )
        .await;
    let audit_closed = signalled.elapsed();
    let status_events = status
        .read_to_end(
            END_BOUND,
            "the server status stream on the api-server process ends after SIGTERM",
        )
        .await;
    let status_closed = signalled.elapsed();
    let exit = api.wait_exit(EXIT_BOUND).await;
    let exited = signalled.elapsed();
    assert!(
        exit.success(),
        "the api-server process exits 0 after its drain: {exit}\n{}",
        api.log()
    );
    assert!(
        status_events.is_empty(),
        "a silent server's status stream closes with no event: {status_events:?}"
    );
    let mut delivered = received(&live);
    // Only rows may precede the end: shutdown adds no event of its own.
    delivered.extend(received(&before_end));
    let last_id = delivered.last().expect("rows were delivered").0;
    println!(
        "api-server process after SIGTERM: audit stream closed in {audit_closed:?}, status stream \
         closed in {status_closed:?}, process exited in {exited:?}"
    );

    // 2. Rows written while no API runs.
    let offline = plant_rows(&pool, &tag, 4).await;
    publish_all(&pool).await;

    // 3. A reconnect with the last id, on a router in this process, replays them exactly once.
    let app = router(state.clone());
    let mut replay = open_router_stream(&app, &token, AUDIT_STREAM_PATH, Some(last_id)).await;
    let ready = replay
        .expect_event(EVENT_BOUND, "ready opens the reconnected stream")
        .await;
    assert_ready(&ready, last_id, retained_floor(&pool).await);
    let expected = publications_after(&pool, last_id).await;
    let expected_ids: Vec<i64> = expected.iter().map(|publication| publication.1).collect();
    assert!(
        offline
            .iter()
            .all(|audit_id| expected_ids.contains(audit_id)),
        "every offline row is published after the last id: {offline:?} in {expected:?}"
    );
    let replayed = replay
        .expect_rows(expected.len(), EVENT_BOUND, "the rows after the last id")
        .await;
    assert_eq!(
        received(&replayed),
        expected,
        "the replay, in sequence order"
    );
    replay
        .expect_quiet(QUIET, "every row after the last id is replayed once")
        .await;
    delivered.extend(received(&replayed));
    assert_eq!(
        delivered,
        publications_after(&pool, opened_after).await,
        "across the shutdown and the reconnect, every row arrives exactly once"
    );

    // 4. This process's shutdown closes its open streams, and a stream opened after it began.
    let mut status = open_router_stream(
        &app,
        &token,
        &server_status_stream_path(register_silent_server(&pool, SUITE).await),
        None,
    )
    .await;
    status
        .expect_quiet(
            QUIET,
            "the in-process status stream is open before shutdown",
        )
        .await;
    let began = Instant::now();
    process_shutdown().begin();
    let after_begin = replay
        .read_to_end(
            END_BOUND,
            "the in-process audit stream ends once shutdown begins",
        )
        .await;
    assert!(
        after_begin.is_empty(),
        "no event on shutdown: {after_begin:?}"
    );
    let status_events = status
        .read_to_end(
            END_BOUND,
            "the in-process status stream ends once shutdown begins",
        )
        .await;
    assert!(
        status_events.is_empty(),
        "no event on shutdown: {status_events:?}"
    );
    let in_process_closed = began.elapsed();
    let mut late = open_router_stream(&app, &token, AUDIT_STREAM_PATH, Some(last_id)).await;
    let late_events = late
        .read_to_end(
            END_BOUND,
            "a stream opened after shutdown began ends at once",
        )
        .await;
    assert!(late_events.is_empty(), "not even ready: {late_events:?}");
    println!("in-process shutdown: both open streams closed in {in_process_closed:?}");
}
