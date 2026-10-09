//! The fixture's own game server: a registered server whose `mod_runtime` credential and runtime
//! session report match results and confirm identity links the way the mod does.
//!
//! The server is created on first use, so suites that never report pay nothing for it.
//! [`Fixture::report_match_results`] registers the body's `source_match_id` on first use and
//! posts the body as that match's next revision, which is how a suite that re-reports a match
//! expresses a correction.

use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Actor, Fixture};

/// The registration start instant every fixture registration uses, so repeats are identical.
const REGISTERED_STARTED_AT: &str = "2026-01-01T00:00:00Z";

/// The fixture server's machine credential and the runtime session it started.
pub(crate) struct ReportingGameServer {
    pub server_id: Uuid,
    pub credential: Actor,
    pub runtime_session_id: Uuid,
}

impl Fixture {
    /// The fixture's game server, registered with a `mod_runtime` credential and a started
    /// runtime session on first use.
    pub(crate) async fn game_server(&self) -> &ReportingGameServer {
        self.game_server
            .get_or_init(|| async {
                let server_id: Uuid = sqlx::query_scalar(
                    "INSERT INTO servers (name, ip, port, is_active)
                     VALUES ($1, '127.0.0.1'::inet, 2001, true) RETURNING id",
                )
                .bind(self.fresh_id("game-server"))
                .fetch_one(self.pool())
                .await
                .expect("the insert into servers returns its row");
                let (status, issued) = self
                    .call(
                        &self.admin,
                        "POST",
                        &format!("/api/v1/servers/{server_id}/credentials"),
                        Some(json!({ "executor_kind": "mod_runtime", "label": "Fixture runtime" })),
                    )
                    .await;
                assert_eq!(
                    status,
                    StatusCode::CREATED,
                    "issue runtime credential: {issued}"
                );
                let credential = Actor {
                    id: String::new(),
                    token: issued["secret"]
                        .as_str()
                        .expect("the `secret` field is a string")
                        .to_owned(),
                };
                let (status, started) = self
                    .call(&credential, "POST", "/api/v1/game-runtime/sessions", None)
                    .await;
                assert_eq!(
                    status,
                    StatusCode::CREATED,
                    "start runtime session: {started}"
                );
                ReportingGameServer {
                    server_id,
                    credential,
                    runtime_session_id: started["runtime_session_id"]
                        .as_str()
                        .expect("the `runtime_session_id` field is a string")
                        .parse()
                        .expect("the `runtime_session_id` field parses as an id"),
                }
            })
            .await
    }

    /// Post `body` (`{match, players, removed_lines?}`) as the next results revision of its
    /// `source_match_id`, registering the match with the fixture server first when it is new.
    pub(crate) async fn report_match_results(&self, body: Value) -> (StatusCode, Value) {
        let server = self.game_server().await;
        let source = body["match"]["source_match_id"]
            .as_str()
            .expect("a reported match names its source_match_id")
            .trim()
            .to_owned();
        let revision = {
            let mut revisions = self
                .result_revisions
                .lock()
                .expect("the mutex is not poisoned");
            let next = revisions.get(&source).copied().unwrap_or(0) + 1;
            revisions.insert(source.clone(), next);
            next
        };
        if revision == 1 {
            let (status, registered) = self
                .call(
                    &server.credential,
                    "POST",
                    "/api/v1/ingest/matches",
                    Some(json!({
                        "source_match_id": source,
                        "runtime_session_id": server.runtime_session_id,
                        "started_at": REGISTERED_STARTED_AT,
                    })),
                )
                .await;
            assert!(
                status == StatusCode::CREATED || status == StatusCode::OK,
                "register {source}: {status} {registered}"
            );
        }
        let mut body = body;
        body["revision"] = json!(revision);
        self.call(
            &server.credential,
            "POST",
            "/api/v1/ingest/match-results",
            Some(body),
        )
        .await
    }

    /// Confirm an identity link code from the fixture server: `{code, arma_id, arma_character}`.
    pub(crate) async fn confirm_link(&self, body: Value) -> (StatusCode, Value) {
        let server = self.game_server().await;
        self.call(
            &server.credential,
            "POST",
            "/api/v1/ingest/link-confirm",
            Some(body),
        )
        .await
    }
}
