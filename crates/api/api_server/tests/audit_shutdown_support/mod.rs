//! Shutdown harness for the audit replay shutdown suite.
//!
//! **Role:** runs the real `api-server` binary over this binary's private database, opens event
//! streams on it over HTTP and on the in-process router, reads SSE bodies to their end with bounded waits,
//! registers the silent server whose status stream stays quiet, and plants audit rows and reads
//! back their publications.
//! **Position:** compiled into `tests/audit_replay_shutdown.rs` only
//! (`mod audit_shutdown_support;`); it adds no test binary. It reaches the database through the
//! suite's pool, the binary through `CARGO_BIN_EXE_api-server`, and the router through
//! `api_server::router::router`.
//! **Signals & state:** each [`ApiProcess`] owns one child process, killed if dropped while it
//! runs, and its log file; each [`SseReader`] owns one response body and the text of its
//! unfinished event.
//! **Invariants:** every wait is bounded and names what it waited for; a body that fails instead
//! of ending cleanly fails the case; the publication table, read after the fact, is the oracle
//! every delivered sequence is compared with.

mod api_process;
mod audit_rows;
mod sse_reader;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub(crate) use api_process::{ApiProcess, http_client};
pub(crate) use audit_rows::{
    plant_rows, publications_after, publish_all, retained_floor, wait_published,
};
pub(crate) use sse_reader::{SseEvent, SseReader};

/// The live audit feed.
pub(crate) const AUDIT_STREAM_PATH: &str = "/api/v1/admin/audit-logs/stream";

/// The live status feed of `server`; a server with no status row opens a stream with no snapshot
/// that stays open until it is closed, and an unknown server answers 404.
pub(crate) fn server_status_stream_path(server: Uuid) -> String {
    format!("/api/v1/servers/{server}/status/stream")
}

/// Registers an active server with no status row, so its status stream opens with no snapshot;
/// the name carries `suite` and a fresh UUID.
pub(crate) async fn register_silent_server(pool: &PgPool, suite: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active) VALUES ($1, '127.0.0.1', 2001, true) \
         RETURNING id",
    )
    .bind(format!("{suite} silent server {}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("register the silent server")
}

/// Asserts a stream answer: 200 with `text/event-stream`.
fn assert_event_stream(status: StatusCode, content_type: Option<&str>, what: &str) {
    assert_eq!(status, StatusCode::OK, "{what} opens");
    let content_type = content_type.unwrap_or_default();
    assert!(
        content_type.starts_with("text/event-stream"),
        "{what} answers text/event-stream, got {content_type:?}"
    );
}

/// Opens `path` on the `api-server` binary as `token`'s bearer, with `last_event_id` as sent.
pub(crate) async fn open_process_stream(
    api: &ApiProcess,
    client: &reqwest::Client,
    token: &str,
    path: &str,
    last_event_id: Option<i64>,
) -> SseReader {
    let mut request = client.get(api.url(path)).bearer_auth(token);
    if let Some(cursor) = last_event_id {
        request = request.header("last-event-id", cursor.to_string());
    }
    let response = request.send().await.unwrap_or_else(|error| {
        panic!(
            "GET {path} on the api-server process: {error}\n{}",
            api.log()
        )
    });
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    assert_event_stream(response.status(), content_type.as_deref(), path);
    SseReader::from_http(response)
}

/// Opens `path` on the in-process router as `token`'s bearer, with `last_event_id` as sent.
pub(crate) async fn open_router_stream(
    app: &Router,
    token: &str,
    path: &str,
    last_event_id: Option<i64>,
) -> SseReader {
    let mut request = Request::builder()
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    if let Some(cursor) = last_event_id {
        request = request.header("last-event-id", cursor);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).expect("build stream request"))
        .await
        .expect("the router answers every request");
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    assert_event_stream(response.status(), content_type.as_deref(), path);
    SseReader::from_router(response)
}
