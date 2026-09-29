//! The fleet and telemetry part's world: servers in every state, a queued command, a match
//! with stored events, and fresh sessions, claims, credentials and matches per probe.
//!
//! **Role:** implements [`PartWorld`] for the server registry and status routes, the machine
//! credential routes, the fleet command ledger and its executor routes, the game-runtime
//! session and heartbeat routes, and the match telemetry routes.
//!
//! **Position:** mounted by `tests/route_acceptance_fleet_and_telemetry.rs` with `#[path]`; its
//! specs are `specs/fleet_and_telemetry.rs`. Rows are made through the real routes with the
//! world's administrator and machine credentials, except servers, which are inserted directly
//! the way the framework registers its own.
//!
//! **Signals & state:** a counter for unique server names and source match ids.
//!
//! **Invariants:** the executor queue of the machine credentials' server holds `list_players`
//! commands (never process-changing), so a host-agent claim succeeds whenever a fixture queued
//! one; the only process change on it is the console command of `executing-console-command`,
//! claimed right after a drain, and a refused console response leaves that command executing,
//! which blocks every later console claim, so the refused-response probe is that fixture's last
//! user; the administrator command routes act on a separate server, so their
//! commands never reach the executor probes; every consuming fixture (session end, claim,
//! executing report, result, cancel, revocation, deactivation, registration) mints its own rows.

use std::sync::atomic::{AtomicU32, Ordering};

use axum::Router;
use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;
use website_api::core::application_state::AppState;
use website_api::core::http_router;

use crate::route_acceptance_support::actors::Actors;
use crate::route_acceptance_support::requests::{Outgoing, send};
use crate::route_acceptance_support::spec::{Actor, Executor, Role};
use crate::route_acceptance_support::world::{Fixture, PartWorld, WorldCore};
use crate::telemetry_support::match_reports::REGISTERED_STARTED_AT;
use crate::telemetry_support::report_fixtures::{event, line, report};

/// The fleet and telemetry world.
pub struct FleetAndTelemetryWorld {
    /// The server the administrator command routes read and write.
    command_server: Uuid,
    /// A queued command of [`Self::command_server`].
    queued_command: Uuid,
    /// The server the credential routes issue on and revoke from.
    credential_server: Uuid,
    /// A deactivated server.
    deactivated_server: Uuid,
    /// A runtime session of the machine credentials' server that registrations name.
    registration_session: Uuid,
    /// A match of the machine credentials' server with two stored events.
    read_match: Uuid,
    names: AtomicU32,
}

/// The `(method, uri, bearer, body)` of one setup request; it must succeed.
async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    bearer: Option<String>,
    body: Option<&Value>,
) -> (StatusCode, Value) {
    let mut outgoing = Outgoing::new(method, uri);
    if let Some(body) = body {
        outgoing = outgoing.json(body);
    }
    outgoing.bearer = bearer;
    let received = send(app, &outgoing).await;
    assert!(
        received.status.is_success(),
        "fleet and telemetry setup {method} {uri}: {} {}",
        received.status,
        received.excerpt()
    );
    (received.status, received.json().unwrap_or(Value::Null))
}

fn administrator(actors: &Actors) -> Option<String> {
    Some(actors.user(Role::Admin).token.clone())
}

fn machine(actors: &Actors, executor: Executor) -> Option<String> {
    actors.bearer(Actor::Machine(executor))
}

fn uuid_at(body: &Value, pointer: &str) -> Uuid {
    body.pointer(pointer)
        .and_then(Value::as_str)
        .and_then(|raw| raw.parse().ok())
        .unwrap_or_else(|| panic!("no UUID at {pointer} in {body}"))
}

fn integer_at(body: &Value, pointer: &str) -> i64 {
    body.pointer(pointer)
        .and_then(Value::as_i64)
        .unwrap_or_else(|| panic!("no integer at {pointer} in {body}"))
}

async fn insert_server(state: &AppState, name: &str, active: bool) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active) VALUES ($1, '127.0.0.1'::inet, 2302, $2) \
         RETURNING id",
    )
    .bind(name)
    .bind(active)
    .fetch_one(&state.pool)
    .await
    .unwrap_or_else(|error| panic!("register fleet and telemetry server {name}: {error}"))
}

/// Start the next runtime session of the machine credentials' server: `(id, generation)`.
async fn start_session(app: &Router, actors: &Actors) -> (Uuid, i64) {
    let bearer = machine(actors, Executor::ModRuntime);
    let (_, started) = call(app, "POST", "/api/v1/game-runtime/sessions", bearer, None).await;
    (
        uuid_at(&started, "/runtime_session_id"),
        integer_at(&started, "/generation"),
    )
}

fn heartbeat_body(generation: i64, sequence: i64) -> Value {
    json!({"generation": generation, "sequence": sequence, "is_online": true,
        "player_count": 4, "max_players": 64})
}

async fn post_heartbeat(app: &Router, actors: &Actors, session: Uuid, body: &Value) {
    let uri = format!("/api/v1/game-runtime/sessions/{session}/heartbeats");
    let bearer = machine(actors, Executor::ModRuntime);
    call(app, "POST", &uri, bearer, Some(body)).await;
}

/// Queue `request` on `server` as the administrator; answers the command id.
async fn queue_command(app: &Router, actors: &Actors, server: Uuid, request: &Value) -> Uuid {
    let uri = format!("/api/v1/servers/{server}/commands");
    let (_, receipt) = call(app, "POST", &uri, administrator(actors), Some(request)).await;
    uuid_at(&receipt, "/id")
}

async fn register_match(app: &Router, actors: &Actors, session: Uuid, source: &str) -> Uuid {
    let body = json!({"source_match_id": source, "runtime_session_id": session,
        "started_at": REGISTERED_STARTED_AT});
    let bearer = machine(actors, Executor::ModRuntime);
    let (_, answer) = call(app, "POST", "/api/v1/ingest/matches", bearer, Some(&body)).await;
    uuid_at(&answer, "/match_id")
}

fn vehicle_entered(event_id: &str, sequence: i64) -> Value {
    event(
        event_id,
        sequence,
        "vehicle.entered",
        json!({"arma_id": "route-acceptance-arma", "vehicle_prefab": "{ROUTE}Vehicle.et",
            "compartment": "driver"}),
    )
}

impl FleetAndTelemetryWorld {
    fn unique(&self, core: &WorldCore, stem: &str) -> String {
        let n = self.names.fetch_add(1, Ordering::Relaxed);
        format!("route-acceptance-{stem}-{}-{n}", core.actors.namespace)
    }

    fn machine_server(core: &WorldCore) -> Uuid {
        core.actors.machines.server
    }

    /// Queue a `list_players` command on the machine credentials' server and claim the oldest
    /// queued command as its host agent: `(command id, fencing token)`.
    async fn claimed_command(core: &WorldCore) -> (Uuid, i64) {
        let server = Self::machine_server(core);
        queue_command(
            &core.app,
            &core.actors,
            server,
            &json!({"action": "list_players"}),
        )
        .await;
        let bearer = machine(&core.actors, Executor::HostAgent);
        let claim_uri = "/api/v1/fleet-executor/commands/claim";
        let (status, claimed) = call(&core.app, "POST", claim_uri, bearer, Some(&json!({}))).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "a queued command is claimable: {claimed}"
        );
        (
            uuid_at(&claimed, "/command_id"),
            integer_at(&claimed, "/fencing_token"),
        )
    }

    /// A claimed command already reported executing: `(command id, fencing token)`.
    async fn executing_command(core: &WorldCore) -> (Uuid, i64) {
        let (command, token) = Self::claimed_command(core).await;
        let uri = format!("/api/v1/fleet-executor/commands/{command}/executing");
        let bearer = machine(&core.actors, Executor::HostAgent);
        let body = json!({"fencing_token": token});
        call(&core.app, "POST", &uri, bearer, Some(&body)).await;
        (command, token)
    }

    /// A console command on the machine credentials' server, claimed and reported executing:
    /// `(command id, fencing token)`. The host-agent queue is drained first, so the claim takes
    /// this command and no older one.
    async fn executing_console_command(core: &WorldCore) -> (Uuid, i64) {
        Self::drain_claims(core).await;
        let line = json!({"action": "console_command", "arguments": {"line": "#players"}});
        let command =
            queue_command(&core.app, &core.actors, Self::machine_server(core), &line).await;
        let bearer = machine(&core.actors, Executor::HostAgent);
        let claim_uri = "/api/v1/fleet-executor/commands/claim";
        let (status, claimed) = call(&core.app, "POST", claim_uri, bearer, Some(&json!({}))).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "{}: the console command is claimable: {claimed}",
            core.suite
        );
        assert_eq!(
            uuid_at(&claimed, "/command_id"),
            command,
            "{}: the drained queue holds only the console command",
            core.suite
        );
        let token = integer_at(&claimed, "/fencing_token");
        let uri = format!("/api/v1/fleet-executor/commands/{command}/executing");
        let bearer = machine(&core.actors, Executor::HostAgent);
        let body = json!({"fencing_token": token});
        call(&core.app, "POST", &uri, bearer, Some(&body)).await;
        (command, token)
    }

    /// Claim every queued host-agent command of the machine credentials' server.
    async fn drain_claims(core: &WorldCore) {
        for _ in 0..200 {
            let bearer = machine(&core.actors, Executor::HostAgent);
            let uri = "/api/v1/fleet-executor/commands/claim";
            let (status, _) = call(&core.app, "POST", uri, bearer, Some(&json!({}))).await;
            if status == StatusCode::NO_CONTENT {
                return;
            }
        }
        panic!("{}: the host-agent queue did not drain", core.suite);
    }

    fn command_fixture(server: Uuid, command: Uuid) -> Fixture {
        Fixture::new()
            .param("id", server.to_string())
            .param("commandId", command.to_string())
    }

    fn executor_fixture(command: Uuid, body: Value) -> Fixture {
        Fixture::new()
            .param("commandId", command.to_string())
            .body(body)
    }

    async fn fresh_command(&self, core: &WorldCore) -> Uuid {
        let request = json!({"action": "list_players"});
        queue_command(&core.app, &core.actors, self.command_server, &request).await
    }

    /// Issue a host-agent credential on the credential server: its revocation fixture.
    async fn credential_fixture(&self, core: &WorldCore) -> Fixture {
        let uri = format!("/api/v1/servers/{}/credentials", self.credential_server);
        let body = json!({"executor_kind": "host_agent", "label": "Route acceptance revocation"});
        let (_, issued) = call(
            &core.app,
            "POST",
            &uri,
            administrator(&core.actors),
            Some(&body),
        )
        .await;
        Fixture::new()
            .param("id", self.credential_server.to_string())
            .param(
                "credentialId",
                uuid_at(&issued, "/credential/id").to_string(),
            )
            .query("reason=route-acceptance")
    }

    async fn heartbeat_fixture(
        core: &WorldCore,
        generation_offset: i64,
        replayed: bool,
    ) -> Fixture {
        let (session, generation) = start_session(&core.app, &core.actors).await;
        let body = heartbeat_body(generation + generation_offset, 1);
        if replayed {
            post_heartbeat(&core.app, &core.actors, session, &body).await;
        }
        Fixture::new()
            .param("sessionId", session.to_string())
            .body(body)
    }

    fn registration(&self, core: &WorldCore) -> Value {
        json!({"source_match_id": self.unique(core, "match"),
            "runtime_session_id": self.registration_session,
            "started_at": REGISTERED_STARTED_AT})
    }

    /// A first results revision of a freshly registered match.
    async fn results(&self, core: &WorldCore) -> Value {
        let source = self.unique(core, "results");
        register_match(&core.app, &core.actors, self.registration_session, &source).await;
        let mut body = report(
            &source,
            "success",
            vec![line("route-acceptance-arma", "slot-1", None)],
        );
        body["revision"] = json!(1);
        body
    }

    /// A one-event batch of a freshly registered match.
    async fn events(&self, core: &WorldCore) -> Value {
        let source = self.unique(core, "events");
        register_match(&core.app, &core.actors, self.registration_session, &source).await;
        json!({"source_match_id": source, "events": [vehicle_entered("route-acceptance-1", 1)]})
    }

    async fn ingest(core: &WorldCore, uri: &str, body: &Value) {
        let bearer = machine(&core.actors, Executor::ModRuntime);
        call(&core.app, "POST", uri, bearer, Some(body)).await;
    }

    /// The fixture of `key` for a probe sent by `actor`.
    async fn fixture_of(&self, core: &WorldCore, key: &str, actor: Actor) -> Option<Fixture> {
        let machine_server = Self::machine_server(core).to_string();
        let fixture = match key {
            "GET /api/v1/servers/{id}/status" | "GET /api/v1/servers/{id}/status/stream" => {
                Fixture::new().param("id", machine_server)
            }
            "deactivated-server" => Fixture::new().param("id", self.deactivated_server.to_string()),
            "POST /api/v1/servers" => Fixture::new().body(json!({
                "name": self.unique(core, "server"), "ip": "127.0.0.1", "port": 2302
            })),
            "PATCH /api/v1/servers/{id}" | "DELETE /api/v1/servers/{id}" => {
                let server = insert_server(&core.state, &self.unique(core, "server"), true).await;
                let fixture = Fixture::new().param("id", server.to_string());
                if key.starts_with("PATCH") {
                    fixture.body(json!({"name": self.unique(core, "renamed")}))
                } else {
                    fixture
                }
            }
            "POST /api/v1/servers/{id}/commands" => Fixture::new()
                .param("id", self.command_server.to_string())
                .body(json!({"action": "list_players"})),
            "GET /api/v1/servers/{id}/commands" => {
                Fixture::new().param("id", self.command_server.to_string())
            }
            "GET /api/v1/servers/{id}/commands/{commandId}" => {
                Self::command_fixture(self.command_server, self.queued_command)
            }
            "command-of-other-server" => {
                Self::command_fixture(self.credential_server, self.queued_command)
            }
            "POST /api/v1/servers/{id}/commands/{commandId}/cancel" => {
                Self::command_fixture(self.command_server, self.fresh_command(core).await)
            }
            "cancelled-command" => {
                let command = self.fresh_command(core).await;
                let uri = format!(
                    "/api/v1/servers/{}/commands/{command}/cancel",
                    self.command_server
                );
                call(&core.app, "POST", &uri, administrator(&core.actors), None).await;
                Self::command_fixture(self.command_server, command)
            }
            "claimed-command" => {
                let (command, _) = Self::claimed_command(core).await;
                Self::command_fixture(Self::machine_server(core), command)
            }
            "POST /api/v1/fleet-executor/commands/claim" => {
                queue_command(
                    &core.app,
                    &core.actors,
                    Self::machine_server(core),
                    &json!({"action": "list_players"}),
                )
                .await;
                if actor == Actor::Machine(Executor::ModRuntime) {
                    let (session, _) = start_session(&core.app, &core.actors).await;
                    Fixture::new().body(json!({"runtime_session_id": session}))
                } else {
                    Fixture::new().body(json!({}))
                }
            }
            "claim-drained" => {
                Self::drain_claims(core).await;
                Fixture::new().body(json!({}))
            }
            "POST /api/v1/fleet-executor/commands/{commandId}/executing" => {
                let (command, token) = Self::claimed_command(core).await;
                Self::executor_fixture(command, json!({"fencing_token": token}))
            }
            "executing-stale-token" => {
                let (command, token) = Self::claimed_command(core).await;
                Self::executor_fixture(command, json!({"fencing_token": token + 1}))
            }
            "executing-command" => {
                let (command, token) = Self::executing_command(core).await;
                Self::executor_fixture(command, json!({"fencing_token": token}))
            }
            "POST /api/v1/fleet-executor/commands/{commandId}/result" => {
                let (command, token) = Self::executing_command(core).await;
                let outcome = json!({"players": [{"arma_id": "route-acceptance-arma",
                    "name": "Route Acceptance"}]});
                Self::executor_fixture(
                    command,
                    json!({"fencing_token": token, "succeeded": true, "outcome": outcome}),
                )
            }
            "executing-console-command" => {
                let (command, token) = Self::executing_console_command(core).await;
                Self::executor_fixture(command, json!({"fencing_token": token}))
            }
            "result-before-executing" => {
                let (command, token) = Self::claimed_command(core).await;
                Self::executor_fixture(command, json!({"fencing_token": token, "succeeded": true}))
            }
            "GET /api/v1/servers/{id}/credentials" => Fixture::new().param("id", machine_server),
            "POST /api/v1/servers/{id}/credentials" => Fixture::new()
                .param("id", self.credential_server.to_string())
                .body(json!({"executor_kind": "host_agent", "label": "Route acceptance"})),
            "DELETE /api/v1/servers/{id}/credentials/{credentialId}" => {
                self.credential_fixture(core).await
            }
            "revoked-credential" => {
                let fixture = self.credential_fixture(core).await;
                let (server, credential) = (&fixture.params[0].1, &fixture.params[1].1);
                let uri = format!("/api/v1/servers/{server}/credentials/{credential}?reason=first");
                call(&core.app, "DELETE", &uri, administrator(&core.actors), None).await;
                fixture
            }
            "POST /api/v1/game-runtime/sessions" => Fixture::new().body(json!({})),
            "POST /api/v1/game-runtime/sessions/{sessionId}/end" => {
                let (session, _) = start_session(&core.app, &core.actors).await;
                Fixture::new().param("sessionId", session.to_string())
            }
            "ended-session" => {
                let (session, _) = start_session(&core.app, &core.actors).await;
                let uri = format!("/api/v1/game-runtime/sessions/{session}/end");
                let bearer = machine(&core.actors, Executor::ModRuntime);
                call(&core.app, "POST", &uri, bearer, None).await;
                Fixture::new().param("sessionId", session.to_string())
            }
            "superseded-session" => {
                let (session, _) = start_session(&core.app, &core.actors).await;
                start_session(&core.app, &core.actors).await;
                Fixture::new().param("sessionId", session.to_string())
            }
            "POST /api/v1/game-runtime/sessions/{sessionId}/heartbeats" => {
                Self::heartbeat_fixture(core, 0, false).await
            }
            "heartbeat-stale-generation" => Self::heartbeat_fixture(core, 1, false).await,
            "heartbeat-replayed" => Self::heartbeat_fixture(core, 0, true).await,
            "heartbeat-fence-only" => {
                let (session, generation) = start_session(&core.app, &core.actors).await;
                Fixture::new()
                    .param("sessionId", session.to_string())
                    .body(json!({"generation": generation, "sequence": 1}))
            }
            "POST /api/v1/ingest/matches" => Fixture::new().body(self.registration(core)),
            "registration-repeated" => {
                let body = self.registration(core);
                Self::ingest(core, "/api/v1/ingest/matches", &body).await;
                Fixture::new().body(body)
            }
            "POST /api/v1/ingest/match-results" => Fixture::new().body(self.results(core).await),
            "results-applied" => {
                let body = self.results(core).await;
                Self::ingest(core, "/api/v1/ingest/match-results", &body).await;
                Fixture::new().body(body)
            }
            "results-superseded" => {
                let mut body = self.results(core).await;
                body["revision"] = json!(2);
                Self::ingest(core, "/api/v1/ingest/match-results", &body).await;
                body["revision"] = json!(1);
                Fixture::new().body(body)
            }
            "results-unregistered" => {
                let source = self.unique(core, "unregistered");
                let mut body = report(&source, "success", Vec::new());
                body["revision"] = json!(1);
                Fixture::new().body(body)
            }
            "POST /api/v1/ingest/match-events" => Fixture::new().body(self.events(core).await),
            "events-stored" => {
                let body = self.events(core).await;
                Self::ingest(core, "/api/v1/ingest/match-events", &body).await;
                Fixture::new().body(body)
            }
            "events-oversized-batch" => {
                let mut body = self.events(core).await;
                body["events"] = Value::Array(
                    (1..=501)
                        .map(|sequence| {
                            vehicle_entered(&format!("route-acceptance-{sequence}"), sequence)
                        })
                        .collect(),
                );
                Fixture::new().body(body)
            }
            "GET /api/v1/matches/{matchId}/events" => {
                Fixture::new().param("matchId", self.read_match.to_string())
            }
            _ => return None,
        };
        Some(fixture)
    }
}

impl PartWorld for FleetAndTelemetryWorld {
    async fn build(state: &mut AppState, actors: &Actors) -> Self {
        let app = http_router::router(state.clone());
        let namespace = actors.namespace;
        let command_server = insert_server(
            state,
            &format!("Route acceptance commands {namespace}"),
            true,
        )
        .await;
        let credential_server = insert_server(
            state,
            &format!("Route acceptance credentials {namespace}"),
            true,
        )
        .await;
        let deactivated_server = insert_server(
            state,
            &format!("Route acceptance deactivated {namespace}"),
            false,
        )
        .await;
        let (registration_session, generation) = start_session(&app, actors).await;
        post_heartbeat(
            &app,
            actors,
            registration_session,
            &heartbeat_body(generation, 1),
        )
        .await;
        let queued_command = queue_command(
            &app,
            actors,
            command_server,
            &json!({"action": "list_players"}),
        )
        .await;
        let source = format!("route-acceptance-read-{namespace}");
        let read_match = register_match(&app, actors, registration_session, &source).await;
        let events = json!({"source_match_id": source, "events": [
            vehicle_entered("route-acceptance-read-1", 1),
            vehicle_entered("route-acceptance-read-2", 2),
        ]});
        let bearer = machine(actors, Executor::ModRuntime);
        call(
            &app,
            "POST",
            "/api/v1/ingest/match-events",
            bearer,
            Some(&events),
        )
        .await;
        FleetAndTelemetryWorld {
            command_server,
            queued_command,
            credential_server,
            deactivated_server,
            registration_session,
            read_match,
            names: AtomicU32::new(1),
        }
    }

    async fn fixture(&self, core: &WorldCore, key: &str, actor: Actor) -> Option<Fixture> {
        self.fixture_of(core, key, actor).await
    }
}
