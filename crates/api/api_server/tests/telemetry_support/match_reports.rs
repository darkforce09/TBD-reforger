//! A game server that reports match telemetry the way the mod does: machine credential, match
//! registration, numbered results revisions and event batches.
//!
//! [`ReportingServer::open`] registers a server and starts its runtime session.
//! [`ReportingServer::report_results`] registers the body's `source_match_id` on first use (with a
//! fixed registration, so repeats are inert) and posts the body as the next revision of that
//! match, which is how a suite that re-posts a report expresses a correction.
//! [`ReportingServer::post_results`] posts an explicit revision, for suites that assert the
//! revision rules themselves.

use std::collections::HashMap;
use std::sync::Mutex;

use axum::Router;
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use super::{RuntimeSession, call, runtime_session};

/// The registration start instant every helper registration uses, so repeats are identical.
pub(crate) const REGISTERED_STARTED_AT: &str = "2026-01-01T00:00:00Z";

/// One registered server with an open runtime session and its per-match revision counters.
pub(crate) struct ReportingServer {
    pub server_id: Uuid,
    pub session: RuntimeSession,
    revisions: Mutex<HashMap<String, i64>>,
}

impl ReportingServer {
    /// Register an active server named `name` and start its runtime session.
    pub(crate) async fn open(app: &Router, pool: &PgPool, name: &str) -> Self {
        let server_id: Uuid = sqlx::query_scalar(
            "INSERT INTO servers (name, ip, port, is_active) VALUES ($1, '127.0.0.1', 2001, true)
             RETURNING id",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("register server");
        let session = runtime_session(app, pool, server_id).await;
        Self {
            server_id,
            session,
            revisions: Mutex::new(HashMap::new()),
        }
    }

    /// A machine-authenticated POST of `body` to `uri`.
    pub(crate) async fn post(&self, app: &Router, uri: &str, body: &Value) -> (StatusCode, Value) {
        call(
            app,
            "POST",
            uri,
            Some(&self.session.secret),
            None,
            Some(&body.to_string()),
        )
        .await
    }

    /// Register `source` with the fixed helper registration; answers the match id.
    pub(crate) async fn register_match(&self, app: &Router, source: &str) -> Uuid {
        let (status, answer) = self.register_match_with(app, source, json!({})).await;
        assert!(
            status == StatusCode::CREATED || status == StatusCode::OK,
            "register {source}: {status} {answer}"
        );
        answer["match_id"]
            .as_str()
            .expect("the `match_id` field is a string")
            .parse()
            .expect("the `match_id` field parses as an id")
    }

    /// Register `source` with `extra` registration fields merged over the fixed ones.
    pub(crate) async fn register_match_with(
        &self,
        app: &Router,
        source: &str,
        extra: Value,
    ) -> (StatusCode, Value) {
        let mut body = json!({
            "source_match_id": source,
            "runtime_session_id": self.session.id,
            "started_at": REGISTERED_STARTED_AT,
        });
        for (key, value) in extra.as_object().expect("extra is an object") {
            body[key] = value.clone();
        }
        self.post(app, "/api/v1/ingest/matches", &body).await
    }

    /// Post `body` (`{match, players, removed_lines?}`) as revision `revision`.
    pub(crate) async fn post_results(
        &self,
        app: &Router,
        revision: i64,
        body: &Value,
    ) -> (StatusCode, Value) {
        let mut body = body.clone();
        body["revision"] = json!(revision);
        self.post(app, "/api/v1/ingest/match-results", &body).await
    }

    /// Register the body's source match if needed and post the body as its next revision.
    pub(crate) async fn report_results(&self, app: &Router, body: &Value) -> (StatusCode, Value) {
        let source = body["match"]["source_match_id"]
            .as_str()
            .expect("a reported match names its source_match_id")
            .trim()
            .to_owned();
        let revision = {
            let mut revisions = self.revisions.lock().expect("the mutex is not poisoned");
            let next = revisions.get(&source).copied().unwrap_or(0) + 1;
            revisions.insert(source.clone(), next);
            next
        };
        if revision == 1 {
            self.register_match(app, &source).await;
        }
        self.post_results(app, revision, body).await
    }

    /// Post one event batch of `source`.
    pub(crate) async fn post_events(
        &self,
        app: &Router,
        source: &str,
        events: Value,
    ) -> (StatusCode, Value) {
        self.post(
            app,
            "/api/v1/ingest/match-events",
            &json!({ "source_match_id": source, "events": events }),
        )
        .await
    }
}
