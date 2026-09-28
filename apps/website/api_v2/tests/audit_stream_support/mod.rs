//! Live audit stream harness for the audit replay, audit query recovery, audit failure-injection
//! and audit controlled-race suites.
//!
//! **Role:** boots the real router over this binary's private database, opens
//! `GET /api/v1/admin/audit-logs/stream` with an optional `Last-Event-ID`, parses its SSE body with
//! bounded waits, drives `audit_delivery_stream` directly, and plants and inspects audit rows,
//! publications and the retained floor.
//! **Position:** compiled into `tests/audit_replay.rs`, `tests/audit_query_recovery.rs`,
//! `tests/failure_injection_audit.rs` and `tests/controlled_races_audit.rs`
//! (`mod audit_stream_support;`); it adds no test binary. It reaches the database through
//! `tests/common` and the API through `website_api::core::http_router::router`.
//! **Signals & state:** [`SUITE_LOCK`] serialises the cases of one binary, which share one
//! database, one publication sequence and one retained floor; each [`SseReader`] owns one response
//! body and the bytes of its unfinished event.
//! **Invariants:** every wait is bounded and names what it waited for; a stream that ends or fails
//! while a case waits on it fails the case; the publication table, read after the fact, is the
//! oracle every delivered sequence is compared with.

#![allow(dead_code)]

use std::time::{Duration, Instant};

use axum::Router;
use axum::body::{Body, BodyDataStream, to_bytes};
use axum::http::{HeaderValue, Request, StatusCode, header};
use futures::{Stream, StreamExt};
use serde_json::Value;
use sqlx::{Executor, PgPool, Postgres};
use tokio::sync::{Mutex, MutexGuard, broadcast, mpsc};
use tokio::time::timeout;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::administration::services::audit_delivery::AuditStreamItem;
use website_api::administration::services::audit_notifier::{AuditNotify, AuditSignal};
use website_api::administration::services::audit_publication::publish_audit_batch;
use website_api::core::{
    application_state::AppState, configuration::Config, database, http_router,
};

use crate::common;

/// The contract every stream event's data is validated against.
pub const AUDIT_SCHEMA: &str = "audit-log.schema.json";

/// The audit table's own name.
pub const AUDIT_TABLE: &str = "audit_logs";

/// The name the audit table carries while a case withholds it from the replay read.
pub const WITHHELD_AUDIT_TABLE: &str = "audit_logs_withheld";

/// Serialises the cases of one test binary: they share the publication sequence and the floor.
static SUITE_LOCK: Mutex<()> = Mutex::const_new(());

/// Hold this for the whole case. A panicking case releases it while unwinding.
pub async fn serialise_case() -> MutexGuard<'static, ()> {
    SUITE_LOCK.lock().await
}

/// A unique `action` for the rows one case plants.
pub fn case_tag(case: &str) -> String {
    format!("audit_stream.{case}.{}", Uuid::new_v4())
}

/// The router, its pool and listener, and an administrator's access token.
pub struct AuditHarness {
    pub state: AppState,
    pub app: Router,
    pub pool: PgPool,
    pub notify: AuditNotify,
    pub admin_token: String,
}

impl AuditHarness {
    /// Boots the router over this binary's database and waits until the audit listener is up.
    pub async fn boot(suite: &str) -> Self {
        let url = common::require_test_database_url()
            .expect("the audit stream suites require their PostgreSQL database");
        let pool = database::connect(&url)
            .await
            .expect("connect test database");
        database::migrate(&pool)
            .await
            .expect("migrate test database");
        let state = AppState::new(pool.clone(), Config::for_tests(url, "audit-stream-secret"));
        let app = http_router::router(state.clone());
        let admin = format!("{suite}-admin-{}", Uuid::new_v4());
        let admin_token = common::access_token(&state, suite, &admin, "admin", true).await;
        let notify = AuditNotify::for_pool(&pool);
        wait_listening(&notify).await;
        Self {
            state,
            app,
            pool,
            notify,
            admin_token,
        }
    }

    /// `GET /api/v1/admin/audit-logs/stream` as the administrator, with `last_event_id` as sent.
    pub async fn request_stream(
        &self,
        last_event_id: Option<HeaderValue>,
    ) -> axum::response::Response {
        let mut request = Request::builder()
            .uri("/api/v1/admin/audit-logs/stream")
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.admin_token),
            );
        if let Some(value) = last_event_id {
            request = request.header("last-event-id", value);
        }
        let request = request.body(Body::empty()).expect("build stream request");
        self.app
            .clone()
            .oneshot(request)
            .await
            .expect("the router answers every request")
    }

    /// Opens the stream, asserting a 200 event stream; `resume_after` becomes `Last-Event-ID`.
    pub async fn open_stream(&self, resume_after: Option<i64>) -> SseReader {
        let response = self
            .request_stream(resume_after.map(HeaderValue::from))
            .await;
        assert_eq!(response.status(), StatusCode::OK, "the stream opens");
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        assert!(
            content_type.starts_with("text/event-stream"),
            "the stream answers text/event-stream, got {content_type:?}"
        );
        SseReader::new(response.into_body())
    }

    /// A GET answered as JSON: the status and the decoded body.
    pub async fn get_json(&self, uri: &str) -> (StatusCode, Value) {
        let request = Request::builder()
            .uri(uri)
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.admin_token),
            )
            .body(Body::empty())
            .expect("build request");
        let response = self
            .app
            .clone()
            .oneshot(request)
            .await
            .expect("the router answers every request");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
}

/// One SSE event: its name (`None` for an audit row), its id and its data.
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: Option<String>,
    pub id: Option<String>,
    pub data: String,
}

impl SseEvent {
    /// The event id as a publication sequence.
    pub fn sequence(&self) -> i64 {
        self.id
            .as_deref()
            .and_then(|id| id.parse().ok())
            .unwrap_or_else(|| panic!("event without a numeric id: {self:?}"))
    }

    /// The event data as JSON.
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.data)
            .unwrap_or_else(|error| panic!("event data is not JSON ({error}): {self:?}"))
    }

    /// `true` for an audit row: an unnamed event.
    pub fn is_row(&self) -> bool {
        self.event.is_none()
    }
}

/// Reads SSE events from a response body frame by frame.
pub struct SseReader {
    body: BodyDataStream,
    pending: String,
}

impl SseReader {
    pub fn new(body: Body) -> Self {
        Self {
            body: body.into_data_stream(),
            pending: String::new(),
        }
    }

    /// The next event within `bound`, or `None` when none completes in time. Comment-only blocks
    /// (keep-alives) are skipped; the end of the body or a body error fails the case.
    pub async fn next_event(&mut self, bound: Duration) -> Option<SseEvent> {
        let deadline = Instant::now() + bound;
        loop {
            while let Some(end) = self.pending.find("\n\n") {
                let block: String = self.pending.drain(..end + 2).collect();
                if let Some(event) = parse_block(&block) {
                    return Some(event);
                }
            }
            let left = deadline.saturating_duration_since(Instant::now());
            match timeout(left, self.body.next()).await {
                Err(_) => return None,
                Ok(None) => panic!("the stream ended; unparsed tail {:?}", self.pending),
                Ok(Some(Err(error))) => panic!("the stream body failed: {error}"),
                Ok(Some(Ok(bytes))) => self
                    .pending
                    .push_str(std::str::from_utf8(&bytes).expect("SSE is UTF-8")),
            }
        }
    }

    /// The next event, failing the case when none arrives within `bound`.
    pub async fn expect_event(&mut self, bound: Duration, why: &str) -> SseEvent {
        match self.next_event(bound).await {
            Some(event) => event,
            None => panic!("no event within {bound:?}: {why}"),
        }
    }

    /// The next event, which must be an audit row.
    pub async fn expect_row(&mut self, bound: Duration, why: &str) -> SseEvent {
        let event = self.expect_event(bound, why).await;
        assert!(
            event.is_row(),
            "expected an audit row ({why}), got {event:?}"
        );
        event
    }

    /// The next `count` events, every one an audit row, all within `bound`.
    pub async fn expect_rows(&mut self, count: usize, bound: Duration, why: &str) -> Vec<SseEvent> {
        let deadline = Instant::now() + bound;
        let mut rows = Vec::with_capacity(count);
        while rows.len() < count {
            let left = deadline.saturating_duration_since(Instant::now());
            let Some(event) = self.next_event(left).await else {
                panic!("{} of {count} rows within {bound:?}: {why}", rows.len());
            };
            assert!(
                event.is_row(),
                "expected row {} ({why}), got {event:?}",
                rows.len()
            );
            rows.push(event);
        }
        rows
    }

    /// Asserts no event arrives within `bound`.
    pub async fn expect_quiet(&mut self, bound: Duration, why: &str) {
        if let Some(event) = self.next_event(bound).await {
            panic!("expected no event within {bound:?} ({why}), got {event:?}");
        }
    }
}

/// One `\n\n`-terminated SSE block as an event, or `None` for a comment-only block.
fn parse_block(block: &str) -> Option<SseEvent> {
    let mut event = None;
    let mut id = None;
    let mut data: Vec<&str> = Vec::new();
    let mut fields = 0;
    for line in block.lines().filter(|line| !line.is_empty()) {
        if line.starts_with(':') {
            continue;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        fields += 1;
        match field {
            "event" => event = Some(value.to_owned()),
            "id" => id = Some(value.to_owned()),
            "data" => data.push(value),
            _ => {}
        }
    }
    (fields > 0).then(|| SseEvent {
        event,
        id,
        data: data.join("\n"),
    })
}

/// Drives a delivery stream on its own task, the way a connected client keeps polling, and
/// forwards every item; the task ends when the receiver is dropped.
pub fn drive<S>(items: S) -> mpsc::UnboundedReceiver<AuditStreamItem>
where
    S: Stream<Item = AuditStreamItem> + Send + 'static,
{
    let (sender, receiver) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let mut items = Box::pin(items);
        while let Some(item) = items.next().await {
            if sender.send(item).is_err() {
                return;
            }
        }
    });
    receiver
}

/// The next forwarded item within `bound`, or `None`; a stream that ended fails the case.
pub async fn next_item(
    items: &mut mpsc::UnboundedReceiver<AuditStreamItem>,
    bound: Duration,
) -> Option<AuditStreamItem> {
    match timeout(bound, items.recv()).await {
        Err(_) => None,
        Ok(None) => panic!("the delivery stream ended"),
        Ok(Some(item)) => Some(item),
    }
}

/// Bounded wait for the listener to be up.
pub async fn wait_listening(notify: &AuditNotify) {
    let bound = Duration::from_secs(10);
    let settle = async {
        while !notify.is_listening() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    };
    timeout(bound, settle)
        .await
        .unwrap_or_else(|_| panic!("audit listener not up within {bound:?}"));
}

/// Bounded wait for `wanted` on `receiver`, skipping every other signal and any lag.
pub async fn wait_signal(
    receiver: &mut broadcast::Receiver<AuditSignal>,
    wanted: AuditSignal,
    bound: Duration,
    why: &str,
) {
    let deadline = Instant::now() + bound;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match timeout(left, receiver.recv()).await {
            Ok(Ok(signal)) if signal == wanted => return,
            Ok(Ok(_)) | Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(broadcast::error::RecvError::Closed)) => {
                panic!("signal channel closed while waiting for {wanted:?}: {why}")
            }
            Err(_) => panic!("no {wanted:?} within {bound:?}: {why}"),
        }
    }
}

/// Plants one audit row in `relation` (see [`AUDIT_TABLE`], [`WITHHELD_AUDIT_TABLE`]). Even
/// ordinals carry an actor and odd ones are system rows, so both row shapes reach the stream.
pub async fn plant_row_in<'e, E>(
    executor: E,
    relation: &'static str,
    tag: &str,
    ordinal: i64,
) -> i64
where
    E: Executor<'e, Database = Postgres>,
{
    let sql = format!(
        "INSERT INTO {relation} (severity, actor_id, actor_name, action, message, target_type, \
         target_id, metadata, created_at) VALUES (CASE $2 % 3 WHEN 0 THEN 'info' WHEN 1 THEN \
         'warn' ELSE 'crit' END::audit_severity, CASE WHEN $2 % 2 = 0 THEN 'audit-stream-actor' END, \
         CASE WHEN $2 % 2 = 0 THEN 'Audit Stream Actor' END, $1, 'audit stream row ' || $2, \
         'audit_stream', $1, jsonb_build_object('ordinal', $2), now()) RETURNING id"
    );
    sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(tag)
        .bind(ordinal)
        .fetch_one(executor)
        .await
        .unwrap_or_else(|error| panic!("plant audit row {ordinal} of {tag}: {error}"))
}

/// Plants one audit row in the audit table.
pub async fn plant_row<'e, E>(executor: E, tag: &str, ordinal: i64) -> i64
where
    E: Executor<'e, Database = Postgres>,
{
    plant_row_in(executor, AUDIT_TABLE, tag, ordinal).await
}

/// Plants `count` audit rows in `relation` with one statement, one commit and one notification
/// per row; answers their ids in ascending order.
pub async fn plant_rows_in(
    pool: &PgPool,
    relation: &'static str,
    tag: &str,
    count: i64,
) -> Vec<i64> {
    let sql = format!(
        "INSERT INTO {relation} (severity, actor_id, actor_name, action, message, target_type, \
         target_id, metadata, created_at) SELECT 'info', NULL, NULL, $1, \
         'audit stream row ' || ordinal, 'audit_stream', $1, \
         jsonb_build_object('ordinal', ordinal), now() FROM generate_series(1, $2) AS ordinal \
         RETURNING id"
    );
    let mut ids: Vec<i64> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(tag)
        .bind(count)
        .fetch_all(pool)
        .await
        .unwrap_or_else(|error| panic!("plant {count} audit rows of {tag}: {error}"));
    ids.sort_unstable();
    ids
}

/// Plants `count` audit rows in the audit table with one statement.
pub async fn plant_rows(pool: &PgPool, tag: &str, count: i64) -> Vec<i64> {
    plant_rows_in(pool, AUDIT_TABLE, tag, count).await
}

/// Publishes every pending audit row.
pub async fn publish_all(pool: &PgPool) {
    while publish_audit_batch(pool, 1000)
        .await
        .expect("publish pending audit rows")
        > 0
    {}
}

/// The publication sequence of `audit_id`, if it is published.
pub async fn sequence_of(pool: &PgPool, audit_id: i64) -> Option<i64> {
    sqlx::query_scalar("SELECT sequence FROM audit_publications WHERE audit_id = $1")
        .bind(audit_id)
        .fetch_optional(pool)
        .await
        .expect("read publication sequence")
}

/// Bounded wait until something publishes `audit_id`; answers its sequence.
pub async fn wait_published(pool: &PgPool, audit_id: i64, bound: Duration) -> i64 {
    let deadline = Instant::now() + bound;
    loop {
        if let Some(sequence) = sequence_of(pool, audit_id).await {
            return sequence;
        }
        assert!(
            Instant::now() < deadline,
            "audit row {audit_id} not published within {bound:?}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// `(tail, retained floor)`: `last_sequence` and `retained_after_sequence`.
pub async fn publication_bounds(pool: &PgPool) -> (i64, i64) {
    sqlx::query_as(
        "SELECT last_sequence, retained_after_sequence FROM audit_publication_state \
         WHERE singleton",
    )
    .fetch_one(pool)
    .await
    .expect("read publication bounds")
}

/// Every publication after `cursor` as `(sequence, audit_id)`, in sequence order.
pub async fn publications_after(pool: &PgPool, cursor: i64) -> Vec<(i64, i64)> {
    sqlx::query_as(
        "SELECT sequence, audit_id FROM audit_publications WHERE sequence > $1 \
         ORDER BY sequence",
    )
    .bind(cursor)
    .fetch_all(pool)
    .await
    .expect("read publications")
}
