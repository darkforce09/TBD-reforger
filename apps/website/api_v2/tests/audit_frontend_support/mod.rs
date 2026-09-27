//! The audit logs page protocol as a test client: an SSE frame reader, the merged view, and the
//! fixture that writes, publishes and deletes the suite's audit rows.
//!
//! **Role:** the client half of `tests/audit_frontend.rs`: connect the stream, wait for `ready`,
//! load the history through the list route's keyset pages, merge live rows and history rows by
//! audit id, and reload the history on `reset`.
//! **Position:** drives `GET /api/v1/admin/audit-logs/stream` and `GET /api/v1/admin/audit-logs`
//! through the real router; writes rows through `administration::services::required_audit` and
//! the audited warning route; validates every stream item and list page against
//! `audit-log.schema.json` through `contract_support`.
//! **Signals & state:** [`AuditConsoleFixture`] holds [`SEQUENCE_LOCK`] for its whole life;
//! [`AuditLogsPage`] owns its stream reader, its cursor (the last SSE id) and its merged view.
//! **Invariants:** a case counts only rows whose message carries its marker; a stream row's
//! sequence is above the page's cursor; one audit id always carries the same JSON, whichever way
//! it arrives.
//!
//! Compiled into the suite that writes `mod audit_frontend_support;`; it adds no test binary.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::pin::Pin;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes, to_bytes};
use axum::http::{Request, StatusCode, header};
use chrono::{DateTime, Utc};
use futures::{Stream, StreamExt};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::{Mutex, MutexGuard};
use tokio::task::JoinHandle;
use tokio::time::{Instant, timeout};
use tower::ServiceExt;
use uuid::Uuid;
use website_api::administration::models::audit_log::AuditSeverity;
use website_api::administration::services::audit_publication::publish_audit_batch;
use website_api::administration::services::required_audit::{
    append_actor_audit_with_severity, append_system_audit,
};
use website_api::core::{
    application_state::AppState, configuration::Config, database, http_router,
};

use crate::{common, contract_support};

pub const SUITE: &str = "audit_frontend";
pub const SCHEMA: &str = "audit-log.schema.json";

/// The bound on one awaited stream frame and on catching the stream up to the tail.
pub const FRAME_WAIT: Duration = Duration::from_secs(10);

/// The cases of this binary share one publication sequence and one retained floor, and a case
/// that deletes published rows raises the floor under every open stream. Each case holds this
/// lock, so the only resets a case sees are the ones it causes.
static SEQUENCE_LOCK: Mutex<()> = Mutex::const_new(());

/// One case's router, database and administrator, and the marker its audit rows carry.
pub struct AuditConsoleFixture {
    pub state: AppState,
    pub app: Router,
    pub pool: PgPool,
    pub admin_id: String,
    pub admin: String,
    /// Carried by the message of every audit row the case writes; the list route's `q` and the
    /// page's stream filter both select on it.
    pub marker: String,
    _sequence: MutexGuard<'static, ()>,
}

impl AuditConsoleFixture {
    pub async fn boot() -> Self {
        let sequence = SEQUENCE_LOCK.lock().await;
        let url = common::require_test_database_url()
            .expect("the audit frontend suite requires the isolated PostgreSQL test database");
        let pool = database::connect(&url)
            .await
            .expect("connect test database");
        database::migrate(&pool)
            .await
            .expect("migrate test database");
        let state = AppState::new(pool.clone(), Config::for_tests(url, "audit-frontend"));
        let app = http_router::router(state.clone());
        let admin_id = format!("{SUITE}-admin-{}", Uuid::new_v4());
        let admin = common::access_token(&state, SUITE, &admin_id, "admin", true).await;
        Self {
            state,
            app,
            pool,
            admin_id,
            admin,
            marker: format!("audit-frontend-{}", Uuid::new_v4().simple()),
            _sequence: sequence,
        }
    }

    /// Appends one info row for the administrator through `required_audit`, committed in its
    /// own transaction, and answers its id.
    pub async fn write_row(&self, label: &str) -> i64 {
        self.write_row_with_severity(AuditSeverity::Info, label)
            .await
    }

    pub async fn write_row_with_severity(&self, severity: AuditSeverity, label: &str) -> i64 {
        write_marked_row(&self.pool, &self.admin_id, &self.marker, severity, label).await
    }

    /// Appends one system row (no actor) through `required_audit` and answers its id.
    pub async fn write_system_row(&self, label: &str) -> i64 {
        self.write_timed_system_row(label).await.0
    }

    /// Appends one system row (no actor) through `required_audit` and answers its id with the
    /// time of the transaction that appended it, the `now()` that transaction reads.
    pub async fn write_timed_system_row(&self, label: &str) -> (i64, DateTime<Utc>) {
        let message = format!("{} {label}", self.marker);
        let mut transaction = self.pool.begin().await.expect("begin system audit");
        let transaction_time: DateTime<Utc> = sqlx::query_scalar("SELECT now()")
            .fetch_one(&mut *transaction)
            .await
            .expect("read the transaction time");
        append_system_audit(
            &mut transaction,
            "audit_frontend.system",
            "suite",
            label,
            &message,
        )
        .await
        .expect("append system audit");
        let id = row_id(&mut transaction, &message).await;
        transaction.commit().await.expect("commit system audit");
        (id, transaction_time)
    }

    /// Appends one row in a transaction the caller commits later: its id is allocated now, but
    /// the row is invisible, and unpublished, until the commit.
    pub async fn begin_held_row(&self, label: &str) -> (Transaction<'static, Postgres>, i64) {
        let message = format!("{} {label}", self.marker);
        let mut transaction = self.pool.begin().await.expect("begin held audit");
        append_actor_audit_with_severity(
            &mut transaction,
            AuditSeverity::Info,
            &self.admin_id,
            "audit_frontend.held",
            "user",
            &self.admin_id,
            &message,
        )
        .await
        .expect("append held audit");
        let id = row_id(&mut transaction, &message).await;
        (transaction, id)
    }

    /// Writes `count` rows from a spawned task, `pause` apart, and answers their ids.
    pub fn spawn_writer(&self, label: &str, count: usize, pause: Duration) -> JoinHandle<Vec<i64>> {
        let (pool, actor, marker, label) = (
            self.pool.clone(),
            self.admin_id.clone(),
            self.marker.clone(),
            label.to_owned(),
        );
        tokio::spawn(async move {
            let mut ids = Vec::with_capacity(count);
            for n in 0..count {
                let row_label = format!("{label} {n}");
                ids.push(
                    write_marked_row(&pool, &actor, &marker, AuditSeverity::Info, &row_label).await,
                );
                tokio::time::sleep(pause).await;
            }
            ids
        })
    }

    /// A member the warning route can target.
    pub async fn seed_member(&self) -> String {
        let id = format!("{SUITE}-member-{}", Uuid::new_v4());
        common::seed_user(
            &self.pool,
            &id,
            "Audit Frontend Member",
            &common::unique_arma(SUITE),
            "enlisted",
        )
        .await;
        id
    }

    /// Warns `member` through `POST /api/v1/admin/users/{id}/warnings`, whose audit row carries
    /// the reason, and answers that row's id.
    pub async fn warn_over_http(&self, member: &str, label: &str) -> i64 {
        let reason = format!("{} {label}", self.marker);
        let request = Request::builder()
            .method("POST")
            .uri(format!("/api/v1/admin/users/{member}/warnings"))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.admin))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "reason": reason }).to_string()))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::CREATED,
            "the warning is issued"
        );
        sqlx::query_scalar("SELECT id FROM audit_logs WHERE strpos(message, $1) > 0")
            .bind(&reason)
            .fetch_one(&self.pool)
            .await
            .expect("the warning's audit row")
    }

    /// Publishes every pending row and answers the tail, the newest publication sequence.
    pub async fn publish_everything(&self) -> i64 {
        while publish_audit_batch(&self.pool, 1000)
            .await
            .expect("publish pending audit rows")
            > 0
        {}
        sqlx::query_scalar("SELECT last_sequence FROM audit_publication_state WHERE singleton")
            .fetch_one(&self.pool)
            .await
            .expect("read the publication tail")
    }

    /// Deletes published rows (their publications go with them, and the retained floor rises),
    /// answering the highest publication sequence removed.
    pub async fn delete_published_rows(&self, ids: &[i64]) -> i64 {
        let highest: Option<i64> = sqlx::query_scalar(
            "SELECT max(sequence) FROM audit_publications WHERE audit_id = ANY($1)",
        )
        .bind(ids)
        .fetch_one(&self.pool)
        .await
        .expect("read the doomed publication sequences");
        let highest = highest.expect("the rows to delete are published");
        let deleted = sqlx::query("DELETE FROM audit_logs WHERE id = ANY($1)")
            .bind(ids)
            .execute(&self.pool)
            .await
            .expect("delete published audit rows");
        assert_eq!(deleted.rows_affected(), ids.len() as u64);
        highest
    }

    pub async fn retained_floor(&self) -> i64 {
        sqlx::query_scalar(
            "SELECT retained_after_sequence FROM audit_publication_state WHERE singleton",
        )
        .fetch_one(&self.pool)
        .await
        .expect("read the retained floor")
    }

    /// Every audit id in the database whose message carries the marker.
    pub async fn database_ids(&self) -> BTreeSet<i64> {
        let ids: Vec<i64> =
            sqlx::query_scalar("SELECT id FROM audit_logs WHERE strpos(message, $1) > 0")
                .bind(&self.marker)
                .fetch_all(&self.pool)
                .await
                .expect("read the suite's audit ids");
        ids.into_iter().collect()
    }
}

async fn write_marked_row(
    pool: &PgPool,
    actor: &str,
    marker: &str,
    severity: AuditSeverity,
    label: &str,
) -> i64 {
    let message = format!("{marker} {label}");
    let mut transaction = pool.begin().await.expect("begin audit");
    append_actor_audit_with_severity(
        &mut transaction,
        severity,
        actor,
        "audit_frontend.write",
        "user",
        actor,
        &message,
    )
    .await
    .expect("append audit");
    let id = row_id(&mut transaction, &message).await;
    transaction.commit().await.expect("commit audit");
    id
}

async fn row_id(transaction: &mut Transaction<'static, Postgres>, message: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM audit_logs WHERE message = $1")
        .bind(message)
        .fetch_one(&mut **transaction)
        .await
        .expect("read the appended audit id")
}

/// One server-sent event: its name (`None` for an unnamed row event), id and data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SseFrame {
    pub event: Option<String>,
    pub id: Option<String>,
    pub data: String,
}

/// Reads server-sent events from a streaming response body.
pub struct SseReader {
    body: Pin<Box<dyn Stream<Item = Result<Bytes, axum::Error>> + Send>>,
    buffer: String,
}

impl SseReader {
    pub fn new(body: Body) -> Self {
        Self {
            body: Box::pin(body.into_data_stream()),
            buffer: String::new(),
        }
    }

    /// The next event, skipping comments and keep-alives, or `None` when none arrives within
    /// `within`. The body stream survives a timeout, so reading can resume where it stopped.
    pub async fn next_within(&mut self, within: Duration) -> Option<SseFrame> {
        let deadline = Instant::now() + within;
        loop {
            if let Some(frame) = self.take_buffered() {
                return Some(frame);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            match timeout(left, self.body.next()).await {
                Err(_) => return None,
                Ok(None) => panic!("the audit stream ended"),
                Ok(Some(Err(error))) => panic!("the audit stream failed: {error}"),
                Ok(Some(Ok(chunk))) => self
                    .buffer
                    .push_str(std::str::from_utf8(&chunk).expect("an SSE body is UTF-8")),
            }
        }
    }

    fn take_buffered(&mut self) -> Option<SseFrame> {
        while let Some(end) = self.buffer.find("\n\n") {
            let block: String = self.buffer.drain(..end + 2).collect();
            let mut frame = SseFrame::default();
            let mut data = Vec::new();
            let mut has_field = false;
            for line in block.lines() {
                if line.is_empty() || line.starts_with(':') {
                    continue;
                }
                let (field, value) = line.split_once(':').unwrap_or((line, ""));
                let value = value.strip_prefix(' ').unwrap_or(value);
                has_field = true;
                match field {
                    "event" => frame.event = Some(value.to_owned()),
                    "id" => frame.id = Some(value.to_owned()),
                    "data" => data.push(value),
                    _ => {}
                }
            }
            if has_field {
                frame.data = data.join("\n");
                return Some(frame);
            }
        }
        None
    }
}

/// The rows the page shows, merged by audit id, and where each id arrived from.
#[derive(Debug, Default)]
pub struct MergedView {
    rows: BTreeMap<i64, Value>,
    /// Every id the history pages listed, over the page's whole life.
    pub from_history: BTreeSet<i64>,
    /// Every id the stream delivered, over the page's whole life.
    pub from_stream: BTreeSet<i64>,
    history_rows: BTreeMap<i64, Value>,
    stream_rows: BTreeMap<i64, Value>,
}

impl MergedView {
    /// Shows `row`; an id already shown must carry the same JSON.
    fn merge(&mut self, row: Value, from_stream: bool) {
        let id = row["id"].as_i64().expect("an audit row id");
        if let Some(shown) = self.rows.get(&id) {
            assert_eq!(
                shown, &row,
                "audit id {id} arrives twice with different JSON"
            );
        }
        if from_stream {
            self.from_stream.insert(id);
            self.stream_rows.insert(id, row.clone());
        } else {
            self.from_history.insert(id);
            self.history_rows.insert(id, row.clone());
        }
        self.rows.insert(id, row);
    }

    /// The ids the page shows.
    pub fn ids(&self) -> BTreeSet<i64> {
        self.rows.keys().copied().collect()
    }

    /// The ids that arrived both through the history and through the stream.
    pub fn overlap(&self) -> BTreeSet<i64> {
        self.from_history
            .intersection(&self.from_stream)
            .copied()
            .collect()
    }

    pub fn history_row(&self, id: i64) -> &Value {
        self.history_rows
            .get(&id)
            .unwrap_or_else(|| panic!("audit id {id} was never listed"))
    }

    pub fn stream_row(&self, id: i64) -> &Value {
        self.stream_rows
            .get(&id)
            .unwrap_or_else(|| panic!("audit id {id} was never streamed"))
    }
}

/// What one stream frame did to the page.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamStep {
    /// An audit row at this publication sequence; `shown` when it carries the case's marker.
    Row { sequence: i64, id: i64, shown: bool },
    /// A reset with this data; the page has reloaded its history.
    Reset(Value),
}

/// The audit logs page: one stream, the keyset history, and the merged view.
pub struct AuditLogsPage {
    app: Router,
    bearer: String,
    marker: String,
    page_size: u32,
    stream: SseReader,
    /// The `ready` data the stream opened with.
    pub ready: Value,
    /// The last SSE id received: the publication sequence the stream has delivered up to.
    pub cursor: i64,
    pub resets: Vec<Value>,
    pub view: MergedView,
    /// The ids the history pages of the current load listed, to catch a row on two pages.
    listed_this_load: BTreeSet<i64>,
    pub history_pages_loaded: usize,
}

impl AuditLogsPage {
    /// Connects the stream at the tail and waits for `ready`.
    pub async fn connect(fixture: &AuditConsoleFixture, page_size: u32) -> Self {
        Self::connect_after(fixture, page_size, None).await
    }

    /// Connects the stream with an optional `Last-Event-ID` and waits for `ready`.
    pub async fn connect_after(
        fixture: &AuditConsoleFixture,
        page_size: u32,
        last_event_id: Option<i64>,
    ) -> Self {
        let mut request = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/audit-logs/stream")
            .header(header::AUTHORIZATION, format!("Bearer {}", fixture.admin))
            .header(header::ACCEPT, "text/event-stream");
        if let Some(cursor) = last_event_id {
            request = request.header("last-event-id", cursor.to_string());
        }
        let response = fixture
            .app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "the stream opens");
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        assert!(
            content_type.starts_with("text/event-stream"),
            "the stream is text/event-stream, got {content_type:?}"
        );
        let mut stream = SseReader::new(response.into_body());
        let frame = stream
            .next_within(FRAME_WAIT)
            .await
            .expect("the stream sends ready");
        assert_eq!(
            frame.event.as_deref(),
            Some("ready"),
            "ready comes first: {frame:?}"
        );
        let ready: Value = serde_json::from_str(&frame.data).expect("ready data is JSON");
        contract_support::assert_valid(SCHEMA, Some("AuditStreamReady"), &ready);
        let cursor = ready["resume_after"].as_i64().unwrap();
        assert_eq!(
            frame.id,
            Some(cursor.to_string()),
            "ready's id is its resume_after"
        );
        Self {
            app: fixture.app.clone(),
            bearer: fixture.admin.clone(),
            marker: fixture.marker.clone(),
            page_size,
            stream,
            ready,
            cursor,
            resets: Vec::new(),
            view: MergedView::default(),
            listed_this_load: BTreeSet::new(),
            history_pages_loaded: 0,
        }
    }

    /// Loads the whole history, one keyset page after another.
    pub async fn load_history(&mut self) {
        self.listed_this_load.clear();
        let mut before = self.load_history_page(None).await;
        while let Some(cursor) = before {
            before = self.load_history_page(Some(cursor)).await;
        }
    }

    /// Loads one history page ("load more" when `before` is set), merges it, and answers the
    /// page's `next_cursor`.
    pub async fn load_history_page(&mut self, before: Option<i64>) -> Option<i64> {
        if before.is_none() {
            self.listed_this_load.clear();
        }
        let mut uri = format!(
            "/api/v1/admin/audit-logs?q={}&limit={}",
            self.marker, self.page_size
        );
        if let Some(before) = before {
            uri.push_str(&format!("&before={before}"));
        }
        let request = Request::builder()
            .method("GET")
            .uri(&uri)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.bearer))
            .body(Body::empty())
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let page: Value = serde_json::from_slice(&bytes).expect("a history page is JSON");
        assert_eq!(status, StatusCode::OK, "GET {uri}: {page}");
        contract_support::assert_valid(SCHEMA, None, &page);
        contract_support::assert_valid(SCHEMA, Some("AuditLogPage"), &page);

        let rows = page["data"].as_array().expect("a page's data").clone();
        let mut previous = before;
        for row in &rows {
            contract_support::assert_valid(SCHEMA, Some("AuditLogEntry"), row);
            let id = row["id"].as_i64().unwrap();
            if let Some(previous) = previous {
                assert!(
                    id < previous,
                    "keyset pages descend strictly: {id} after {previous}"
                );
            }
            assert!(
                self.listed_this_load.insert(id),
                "audit id {id} is listed on two pages of one load"
            );
            previous = Some(id);
        }
        let next_cursor = page["next_cursor"].as_i64();
        let full = rows.len() == self.page_size as usize;
        assert_eq!(
            next_cursor,
            full.then(|| rows.last().unwrap()["id"].as_i64().unwrap()),
            "next_cursor is the last id of a full page and null otherwise: {page}"
        );
        for row in rows {
            self.view.merge(row, false);
        }
        self.history_pages_loaded += 1;
        next_cursor
    }

    /// Applies one stream frame, waiting up to [`FRAME_WAIT`] for it.
    pub async fn step(&mut self) -> StreamStep {
        let frame = self
            .stream
            .next_within(FRAME_WAIT)
            .await
            .unwrap_or_else(|| panic!("no stream frame after cursor {}", self.cursor));
        self.apply(frame).await
    }

    /// Applies every frame that arrives within `within` of the previous one.
    pub async fn pump(&mut self, within: Duration) -> Vec<StreamStep> {
        let mut steps = Vec::new();
        while let Some(frame) = self.stream.next_within(within).await {
            steps.push(self.apply(frame).await);
        }
        steps
    }

    /// Applies frames until the stream has delivered through `tail`.
    pub async fn catch_up(&mut self, tail: i64) {
        while self.cursor < tail {
            self.step().await;
        }
    }

    async fn apply(&mut self, frame: SseFrame) -> StreamStep {
        let data: Value = serde_json::from_str(&frame.data)
            .unwrap_or_else(|error| panic!("stream data is JSON ({error}): {frame:?}"));
        let id: i64 = frame
            .id
            .as_deref()
            .and_then(|id| id.parse().ok())
            .unwrap_or_else(|| panic!("every stream event carries a numeric id: {frame:?}"));
        match frame.event.as_deref() {
            None => {
                contract_support::assert_valid(SCHEMA, Some("AuditLogEntry"), &data);
                assert!(
                    id > self.cursor,
                    "rows arrive in ascending sequence: {id} after {}",
                    self.cursor
                );
                self.cursor = id;
                let audit_id = data["id"].as_i64().unwrap();
                let shown = data["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(&self.marker));
                if shown {
                    self.view.merge(data, true);
                }
                StreamStep::Row {
                    sequence: id,
                    id: audit_id,
                    shown,
                }
            }
            Some("reset") => {
                contract_support::assert_valid(SCHEMA, Some("AuditStreamReset"), &data);
                assert_eq!(
                    data["resume_after"],
                    json!(id),
                    "a reset's id is its resume_after"
                );
                self.cursor = id;
                self.resets.push(data.clone());
                self.view.rows.clear();
                self.load_history().await;
                StreamStep::Reset(data)
            }
            Some(other) => panic!("unexpected stream event {other}: {frame:?}"),
        }
    }
}
