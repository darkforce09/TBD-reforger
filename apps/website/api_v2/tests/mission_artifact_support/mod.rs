//! Owned mission fixtures for the artifact, review and deployment suites: suite-owned authors and
//! reviewers, missions with compilable versions, and helpers that drive the real mission,
//! approval and review routes.
//!
//! Compiled into each suite that writes `mod mission_artifact_support;`; it adds no test binary.

#![allow(dead_code)]

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::core::{
    application_state::AppState, configuration::Config, database, http_router,
};

use crate::common;

pub struct Actor {
    pub id: String,
    pub token: String,
}

pub struct MissionFixture {
    pub state: AppState,
    pub app: Router,
    pub author: Actor,
    pub admin: Actor,
    suite: String,
}

/// The compilable payload with one slot's role replaced, so a later version compiles to
/// different bytes.
pub fn payload_with_role(role: &str) -> String {
    common::COMPILABLE_EDITOR_PAYLOAD.replace(r#""role":"SL""#, &format!(r#""role":"{role}""#))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn refusal_code(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

pub fn uuid_of(value: &Value) -> Uuid {
    value
        .as_str()
        .unwrap_or_else(|| panic!("expected a UUID string, got {value}"))
        .parse()
        .unwrap()
}

impl MissionFixture {
    pub async fn new(suite: &str) -> Self {
        let url = common::require_test_database_url().expect("mission suites require PostgreSQL");
        let pool = database::connect(&url).await.unwrap();
        database::migrate(&pool).await.unwrap();
        let state = AppState::new(pool, Config::for_tests(url, "mission-artifacts"));
        let app = http_router::router(state.clone());
        let mut fixture = Self {
            state,
            app,
            author: Actor {
                id: String::new(),
                token: String::new(),
            },
            admin: Actor {
                id: String::new(),
                token: String::new(),
            },
            suite: suite.to_owned(),
        };
        fixture.author = fixture.account("author", "mission_maker").await;
        fixture.admin = fixture.account("reviewer", "admin").await;
        fixture
    }

    pub fn pool(&self) -> &PgPool {
        &self.state.pool
    }

    /// A suite-owned account with a verified membership snapshot for `role`.
    pub async fn account(&self, label: &str, role: &str) -> Actor {
        let id = format!("{}-{label}-{}", self.suite, Uuid::new_v4());
        let token = common::access_token(&self.state, &self.suite, &id, role, true).await;
        Actor { id, token }
    }

    pub async fn call(
        &self,
        actor: Option<&Actor>,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let (status, _, bytes) = self
            .send(actor, method, uri, body.map(|value| value.to_string()))
            .await;
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// One request with a raw body; answers the status, headers and body bytes.
    pub async fn send(
        &self,
        actor: Option<&Actor>,
        method: &str,
        uri: &str,
        body: Option<String>,
    ) -> (StatusCode, HeaderMap, Vec<u8>) {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(actor) = actor {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", actor.token));
        }
        if body.is_some() {
            request = request.header(header::CONTENT_TYPE, "application/json");
        }
        let request = request
            .body(body.map_or(Body::empty(), Body::from))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec();
        (status, headers, bytes)
    }

    /// A draft mission authored by [`Self::author`].
    pub async fn create_mission(&self, title: &str) -> Uuid {
        let (status, body) = self
            .call(
                Some(&self.author),
                "POST",
                "/api/v1/missions",
                Some(json!({
                    "title": title, "terrain": "everon", "game_mode": "pve_coop", "max_players": 16
                })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        uuid_of(&body["id"])
    }

    pub async fn save_version(
        &self,
        actor: &Actor,
        mission: Uuid,
        semver: &str,
        payload: &str,
    ) -> (StatusCode, Value) {
        let (status, _, bytes) = self
            .send(
                Some(actor),
                "POST",
                &format!("/api/v1/missions/{mission}/versions"),
                Some(format!(r#"{{"semver":"{semver}","payload":{payload}}}"#)),
            )
            .await;
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// Save `payload` as the author's version `semver`; answers the version id.
    pub async fn save(&self, mission: Uuid, semver: &str, payload: &str) -> Uuid {
        let (status, body) = self
            .save_version(&self.author, mission, semver, payload)
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        uuid_of(&body["id"])
    }

    /// A draft mission whose current version compiles; answers `(mission, version)`.
    pub async fn compilable_mission(&self, title: &str) -> (Uuid, Uuid) {
        let mission = self.create_mission(title).await;
        let version = self
            .save(mission, "0.2.0", common::COMPILABLE_EDITOR_PAYLOAD)
            .await;
        (mission, version)
    }

    pub async fn submit_as(&self, actor: &Actor, mission: Uuid) -> (StatusCode, Value) {
        self.call(
            Some(actor),
            "POST",
            &format!("/api/v1/missions/{mission}/submit"),
            None,
        )
        .await
    }

    /// Submit as the author and answer the artifact the opened review decides.
    pub async fn submit(&self, mission: Uuid) -> Uuid {
        let (status, body) = self.submit_as(&self.author, mission).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["status"], "pending_approval");
        self.pending_artifact(mission)
            .await
            .expect("a submission opens a review")
    }

    /// The artifact of the mission's pending review, read from the admin approvals queue.
    pub async fn pending_artifact(&self, mission: Uuid) -> Option<Uuid> {
        let row = self.queue_row(mission).await?;
        row.get("artifact_id").map(uuid_of)
    }

    /// The mission's row of `GET /approvals`, walking every page.
    pub async fn queue_row(&self, mission: Uuid) -> Option<Value> {
        let mut offset = 0;
        loop {
            let (status, body) = self
                .call(
                    Some(&self.admin),
                    "GET",
                    &format!("/api/v1/approvals?limit=100&offset={offset}"),
                    None,
                )
                .await;
            assert_eq!(status, StatusCode::OK, "{body}");
            let rows = body["data"].as_array().unwrap();
            if let Some(row) = rows
                .iter()
                .find(|row| row["mission_id"] == mission.to_string())
            {
                return Some(row.clone());
            }
            if rows.len() < 100 {
                return None;
            }
            offset += rows.len();
        }
    }

    pub async fn approve(
        &self,
        mission: Uuid,
        artifact: Uuid,
        conditions: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut body = json!({ "artifact_id": artifact });
        if let Some(conditions) = conditions {
            body["conditions"] = json!(conditions);
        }
        self.call(
            Some(&self.admin),
            "POST",
            &format!("/api/v1/approvals/{mission}/approve"),
            Some(body),
        )
        .await
    }

    pub async fn reject(&self, mission: Uuid, artifact: Uuid, reason: &str) -> (StatusCode, Value) {
        self.call(
            Some(&self.admin),
            "POST",
            &format!("/api/v1/approvals/{mission}/reject"),
            Some(json!({ "artifact_id": artifact, "reason": reason })),
        )
        .await
    }

    pub async fn reviews(&self, actor: &Actor, mission: Uuid) -> (StatusCode, Value) {
        self.call(
            Some(actor),
            "GET",
            &format!("/api/v1/missions/{mission}/reviews"),
            None,
        )
        .await
    }

    pub async fn artifact(
        &self,
        actor: &Actor,
        mission: Uuid,
        artifact: Uuid,
    ) -> (StatusCode, Value) {
        self.call(
            Some(actor),
            "GET",
            &format!("/api/v1/missions/{mission}/artifacts/{artifact}"),
            None,
        )
        .await
    }

    pub async fn mission(&self, actor: &Actor, mission: Uuid) -> Value {
        let (status, body) = self
            .call(
                Some(actor),
                "GET",
                &format!("/api/v1/missions/{mission}"),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }

    pub async fn count(&self, sql: &str, mission: Uuid) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .bind(mission)
            .fetch_one(self.pool())
            .await
            .unwrap()
    }

    /// Audit records of `action` for the mission.
    pub async fn audits(&self, action: &str, mission: Uuid) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM audit_logs WHERE action = $1 AND target_type = 'mission' AND target_id = $2",
        )
        .bind(action)
        .bind(mission.to_string())
        .fetch_one(self.pool())
        .await
        .unwrap()
    }
}

/// A machine caller: the credential secret presented as the bearer.
pub fn machine(secret: &str) -> Actor {
    Actor {
        id: String::new(),
        token: secret.to_owned(),
    }
}

impl MissionFixture {
    /// A compilable mission, submitted and approved; answers `(mission, artifact)`.
    pub async fn approved_mission(&self, title: &str) -> (Uuid, Uuid) {
        let (mission, _) = self.compilable_mission(title).await;
        let artifact = self.submit(mission).await;
        let (status, body) = self.approve(mission, artifact, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        (mission, artifact)
    }

    /// A registered, active server requiring `modpack`.
    pub async fn register_server(&self, name: &str, modpack: Option<Uuid>) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO servers (name, ip, port, is_active, required_modpack_id)
             VALUES ($1, '127.0.0.1'::inet, 2302, true, $2) RETURNING id",
        )
        .bind(name)
        .bind(modpack)
        .fetch_one(self.pool())
        .await
        .unwrap()
    }

    /// Issue a `executor` credential for `server` and answer its secret.
    pub async fn credential(&self, server: Uuid, executor: &str) -> String {
        let (status, body) = self
            .call(
                Some(&self.admin),
                "POST",
                &format!("/api/v1/servers/{server}/credentials"),
                Some(
                    json!({ "executor_kind": executor, "label": format!("{executor} credential") }),
                ),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["secret"].as_str().unwrap().to_owned()
    }

    pub async fn register_scenario(&self, terrain: &str, scenario: &str) {
        let (status, body) = self
            .call(
                Some(&self.admin),
                "PUT",
                &format!("/api/v1/fleet/scenarios/{terrain}"),
                Some(json!({ "scenario_id": scenario, "display_name": terrain })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    /// An upcoming event bound to `server` with `mission` attached; its ORBAT is derived from
    /// the mission's current version. Answers `(event, event mission)`.
    pub async fn event_on(&self, server: Uuid, mission: Uuid) -> (Uuid, Uuid) {
        let (status, event) = self
            .call(
                Some(&self.admin),
                "POST",
                "/api/v1/events",
                Some(json!({ "start_time": "2027-11-01T00:00:00Z" })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{event}");
        let event = uuid_of(&event["id"]);
        sqlx::query("UPDATE events SET server_id = $2 WHERE id = $1")
            .bind(event)
            .bind(server)
            .execute(self.pool())
            .await
            .unwrap();
        let (status, attached) = self
            .call(
                Some(&self.admin),
                "POST",
                &format!("/api/v1/events/{event}/missions"),
                Some(json!({ "mission_id": mission, "start_time": "2027-11-01T00:00:00Z" })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{attached}");
        (event, uuid_of(&attached["id"]))
    }

    pub async fn request_deployment(
        &self,
        actor: &Actor,
        server: Uuid,
        mission: Uuid,
        artifact: Uuid,
        event_mission: Option<Uuid>,
    ) -> (StatusCode, Value) {
        let mut body = json!({ "mission_id": mission, "artifact_id": artifact });
        if let Some(event_mission) = event_mission {
            body["event_mission_id"] = json!(event_mission);
        }
        self.call(
            Some(actor),
            "POST",
            &format!("/api/v1/servers/{server}/deployments"),
            Some(body),
        )
        .await
    }

    /// Request a deployment as the administrator and answer it.
    pub async fn deploy(
        &self,
        server: Uuid,
        mission: Uuid,
        artifact: Uuid,
        event_mission: Option<Uuid>,
    ) -> Value {
        let (status, body) = self
            .request_deployment(&self.admin, server, mission, artifact, event_mission)
            .await;
        assert_eq!(status, StatusCode::ACCEPTED, "{body}");
        body
    }

    /// The server's deployment as an administrator reads it, settled first.
    pub async fn deployment(&self, server: Uuid, deployment: &Value) -> Value {
        let (status, body) = self
            .call(
                Some(&self.admin),
                "GET",
                &format!(
                    "/api/v1/servers/{server}/deployments/{}",
                    deployment["id"].as_str().unwrap()
                ),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }

    /// Start a runtime session reporting `loaded` (`(artifact, sha256)`), or no artifact.
    pub async fn start_session(
        &self,
        secret: &str,
        loaded: Option<(Uuid, &str)>,
    ) -> (StatusCode, Value) {
        let body = loaded.map(|(artifact, sha256)| {
            json!({ "loaded_artifact_id": artifact, "loaded_artifact_sha256": sha256 })
        });
        self.call(
            Some(&machine(secret)),
            "POST",
            "/api/v1/game-runtime/sessions",
            body,
        )
        .await
    }

    /// The document SHA-256 of an artifact.
    pub async fn artifact_sha256(&self, artifact: Uuid) -> String {
        sqlx::query_scalar("SELECT document_sha256 FROM mission_artifacts WHERE id = $1")
            .bind(artifact)
            .fetch_one(self.pool())
            .await
            .unwrap()
    }
}

/// A uniquely named trigger that fails `operation` on `table` for rows whose `column` equals
/// `value`, injecting a real storage failure into one mission's transaction only.
pub async fn inject_failure(
    pool: &PgPool,
    table: &str,
    operation: &str,
    column: &str,
    value: &str,
) -> String {
    let name = format!("mission_failure_{}", Uuid::new_v4().simple());
    assert!(matches!(
        (table, column),
        ("audit_logs", "target_id") | ("mission_review_comments", "mission_id")
    ));
    assert!(value.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    let sql = format!(
        "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            IF NEW.{column}::text = '{value}' THEN RAISE EXCEPTION 'injected mission failure'; END IF;
            RETURN NEW; END; $$;
         CREATE TRIGGER {name} BEFORE {operation} ON {table} FOR EACH ROW EXECUTE FUNCTION {name}();"
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql.as_str()))
        .execute(pool)
        .await
        .unwrap();
    name
}

pub async fn clear_failure(pool: &PgPool, name: &str, table: &str) {
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP TRIGGER {name} ON {table}; DROP FUNCTION {name}();"
    )))
    .execute(pool)
    .await
    .unwrap();
}
