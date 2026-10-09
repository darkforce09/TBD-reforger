//! Live audit stream harness for the audit replay suite.
//!
//! **Role:** boots the real router over the binary's isolated `audit_stream` database, opens
//! `GET /api/v1/admin/audit-logs/stream` with an optional `Last-Event-ID`, parses its SSE body with
//! bounded waits, drives `audit_delivery_stream` directly, and plants and inspects audit rows,
//! publications and the retained floor.
//! **Position:** mounted by `tests/http_infrastructure/main.rs` for its `audit_replay` module; it
//! adds no test binary. It reaches the database through
//! `tests/common` and the API through `api_server::router::router`.
//! **Signals & state:** [`SUITE_LOCK`] serialises the cases, which share one isolated database
//! (a case renames the audit table), one publication sequence and one retained floor; each [`SseReader`] owns one response
//! body and the bytes of its unfinished event.
//! **Invariants:** every wait is bounded and names what it waited for; a stream that ends or fails
//! while a case waits on it fails the case; the publication table, read after the fact, is the
//! oracle every delivered sequence is compared with.

#![allow(dead_code)]

use std::time::{Duration, Instant};

use api_administration::services::audit_notifier::AuditNotify;
use api_administration::services::audit_publication::publish_audit_batch;
use api_configuration::configuration::Config;
use api_server::router::router;
use axum::Router;
use axum::body::{Body, BodyDataStream, to_bytes};
use axum::http::{HeaderValue, Request, StatusCode, header};
use futures::StreamExt;
use serde_json::Value;
use sqlx::{Executor, PgPool, Postgres};
use tokio::sync::{Mutex, MutexGuard};
use tokio::time::timeout;
use tower::ServiceExt;
use uuid::Uuid;

use crate::common;

/// The contract every stream event's data is validated against.
pub(crate) const AUDIT_SCHEMA: &str = "audit-log.schema.json";

/// The audit table's own name.
pub(crate) const AUDIT_TABLE: &str = "audit_logs";

/// Serialises the cases of one test binary: they share the publication sequence and the floor.
static SUITE_LOCK: Mutex<()> = Mutex::const_new(());

/// Hold this for the whole case. A panicking case releases it while unwinding.
pub(crate) async fn serialise_case() -> MutexGuard<'static, ()> {
    SUITE_LOCK.lock().await
}

/// A unique `action` for the rows one case plants.
pub(crate) fn case_tag(case: &str) -> String {
    format!("audit_stream.{case}.{}", Uuid::new_v4())
}

/// The router, its pool and listener, and an administrator's access token.
pub(crate) struct AuditHarness {
    pub app: Router,
    pub pool: PgPool,
    pub admin_token: String,
}

impl AuditHarness {
    /// Boots the router over the binary's isolated `audit_stream` database and waits until the
    /// audit listener is up.
    pub(crate) async fn boot(suite: &str) -> Self {
        let url = common::require_isolated_test_database_url("audit_stream");
        let pool = api_database::connect(&url)
            .await
            .expect("connect test database");
        api_database::migrate(&pool)
            .await
            .expect("migrate test database");
        let state = api_server::composition::application_state(
            pool.clone(),
            Config::for_tests(url, "audit-stream-secret"),
        );
        let app = router(state.clone());
        let admin = format!("{suite}-admin-{}", Uuid::new_v4());
        let admin_token = common::access_token(&state, suite, &admin, "admin", true).await;
        let notify = AuditNotify::for_pool(&pool);
        wait_listening(&notify).await;
        Self {
            app,
            pool,
            admin_token,
        }
    }

    /// `GET /api/v1/admin/audit-logs/stream` as the administrator, with `last_event_id` as sent.
    pub(crate) async fn request_stream(
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
    pub(crate) async fn open_stream(&self, resume_after: Option<i64>) -> SseReader {
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
    pub(crate) async fn get_json(&self, uri: &str) -> (StatusCode, Value) {
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
pub(crate) struct SseEvent {
    pub event: Option<String>,
    pub id: Option<String>,
    pub data: String,
}

impl SseEvent {
    /// The event id as a publication sequence.
    pub(crate) fn sequence(&self) -> i64 {
        self.id
            .as_deref()
            .and_then(|id| id.parse().ok())
            .unwrap_or_else(|| panic!("event without a numeric id: {self:?}"))
    }

    /// The event data as JSON.
    pub(crate) fn json(&self) -> Value {
        serde_json::from_str(&self.data)
            .unwrap_or_else(|error| panic!("event data is not JSON ({error}): {self:?}"))
    }

    /// `true` for an audit row: an unnamed event.
    pub(crate) fn is_row(&self) -> bool {
        self.event.is_none()
    }
}

/// Reads SSE events from a response body frame by frame.
pub(crate) struct SseReader {
    body: BodyDataStream,
    pending: String,
}

impl SseReader {
    pub(crate) fn new(body: Body) -> Self {
        Self {
            body: body.into_data_stream(),
            pending: String::new(),
        }
    }

    /// The next event within `bound`, or `None` when none completes in time. Comment-only blocks
    /// (keep-alives) are skipped; the end of the body or a body error fails the case.
    pub(crate) async fn next_event(&mut self, bound: Duration) -> Option<SseEvent> {
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
    pub(crate) async fn expect_event(&mut self, bound: Duration, why: &str) -> SseEvent {
        match self.next_event(bound).await {
            Some(event) => event,
            None => panic!("no event within {bound:?}: {why}"),
        }
    }

    /// The next event, which must be an audit row.
    pub(crate) async fn expect_row(&mut self, bound: Duration, why: &str) -> SseEvent {
        let event = self.expect_event(bound, why).await;
        assert!(
            event.is_row(),
            "expected an audit row ({why}), got {event:?}"
        );
        event
    }

    /// The next `count` events, every one an audit row, all within `bound`.
    pub(crate) async fn expect_rows(
        &mut self,
        count: usize,
        bound: Duration,
        why: &str,
    ) -> Vec<SseEvent> {
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

/// Bounded wait for the listener to be up.
pub(crate) async fn wait_listening(notify: &AuditNotify) {
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

/// Plants one audit row in `relation` (see [`AUDIT_TABLE`], [`WITHHELD_AUDIT_TABLE`]). Even
/// ordinals carry an actor and odd ones are system rows, so both row shapes reach the stream.
pub(crate) async fn plant_row_in<'e, E>(
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
pub(crate) async fn plant_row<'e, E>(executor: E, tag: &str, ordinal: i64) -> i64
where
    E: Executor<'e, Database = Postgres>,
{
    plant_row_in(executor, AUDIT_TABLE, tag, ordinal).await
}

/// Plants `count` audit rows in `relation` with one statement, one commit and one notification
/// per row; answers their ids in ascending order.
pub(crate) async fn plant_rows_in(
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
pub(crate) async fn plant_rows(pool: &PgPool, tag: &str, count: i64) -> Vec<i64> {
    plant_rows_in(pool, AUDIT_TABLE, tag, count).await
}

/// Publishes every pending audit row.
pub(crate) async fn publish_all(pool: &PgPool) {
    while publish_audit_batch(pool, 1000)
        .await
        .expect("publish pending audit rows")
        > 0
    {}
}

/// The publication sequence of `audit_id`, if it is published.
pub(crate) async fn sequence_of(pool: &PgPool, audit_id: i64) -> Option<i64> {
    sqlx::query_scalar("SELECT sequence FROM audit_publications WHERE audit_id = $1")
        .bind(audit_id)
        .fetch_optional(pool)
        .await
        .expect("read publication sequence")
}

/// `(tail, retained floor)`: `last_sequence` and `retained_after_sequence`.
pub(crate) async fn publication_bounds(pool: &PgPool) -> (i64, i64) {
    sqlx::query_as(
        "SELECT last_sequence, retained_after_sequence FROM audit_publication_state \
         WHERE singleton",
    )
    .fetch_one(pool)
    .await
    .expect("read publication bounds")
}

/// Every publication after `cursor` as `(sequence, audit_id)`, in sequence order.
pub(crate) async fn publications_after(pool: &PgPool, cursor: i64) -> Vec<(i64, i64)> {
    sqlx::query_as(
        "SELECT sequence, audit_id FROM audit_publications WHERE sequence > $1 \
         ORDER BY sequence",
    )
    .bind(cursor)
    .fetch_all(pool)
    .await
    .expect("read publications")
}
