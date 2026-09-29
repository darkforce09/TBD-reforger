//! Route specs of the `fleet_and_telemetry` part: the server registry and its live status,
//! machine credentials, the fleet command ledger and its executor routes, game-runtime
//! sessions and heartbeats, and the match telemetry ingest and read routes.
//!
//! Each spec supplies only what the framework cannot derive — the success status and contract,
//! the fixture-dependent probes, the overrides where an executor route serves both executor
//! kinds, and a reason for every dimension that does not apply. Fixture keys name rows the
//! world `world/fleet_and_telemetry.rs` mints per probe.

use serde_json::json;
use website_api::match_telemetry::models::generated::match_telemetry::{
    MatchEventBatchAnswer, MatchEventPage, MatchRegistrationAnswer, MatchResultsAnswer,
};
use website_api::server_infrastructure::models::generated::fleet_command::{
    ClaimedFleetCommand, FleetCommandList, FleetCommandReceipt,
};
use website_api::server_infrastructure::models::generated::game_runtime_session::{
    RuntimeSessionEnd, StartedRuntimeSession,
};
use website_api::server_infrastructure::models::generated::machine_credential::{
    IssuedMachineCredential, MachineCredential, MachineCredentialList,
};

use super::super::contracts::round_trip;
use super::super::spec::{
    Actor, Change, Contract, Dimension, Executor, Expect, Probe, Role, RouteSpec,
};

const ADMIN: Actor = Actor::User(Role::Admin);
const ENLISTED: Actor = Actor::User(Role::Enlisted);
const RUNTIME: Actor = Actor::Machine(Executor::ModRuntime);
const HOST_AGENT: Actor = Actor::Machine(Executor::HostAgent);
const OTHER_RUNTIME: Actor = Actor::MachineOtherServer(Executor::ModRuntime);
const OTHER_HOST_AGENT: Actor = Actor::MachineOtherServer(Executor::HostAgent);

const FLEET_ADMINISTRATION: &str =
    "administrators manage every server; no account owns a server or its ledger";
const SHARED_INTEL: &str =
    "server intel is shared with every signed-in member; no account owns a server";
const SESSION_OF_CALLER: &str =
    "the machine credential names the server; the route addresses no other resource";
const EXECUTOR_KINDS: &str = "both executor kinds report on the executor routes; a claim \
     belongs to the credential that took it, so another credential's report is a stale \
     fencing token (fleet-command.schema.json ExecutionStart)";
/// A UUID no world ever stores: an unknown session, artifact, modpack or server id.
const UNSTORED_ID: &str = "00000000-0000-4000-8000-00000000f5e5";

fn server_intel(definition: &'static str) -> Contract {
    Contract::schema("server-intel.schema.json", definition)
}

fn fleet_command(definition: &'static str) -> Contract {
    Contract::schema("fleet-command.schema.json", definition)
}

fn machine_credential(definition: &'static str) -> Contract {
    Contract::schema("machine-credential.schema.json", definition)
}

fn runtime_session(definition: &'static str) -> Contract {
    Contract::schema("game-runtime-session.schema.json", definition)
}

fn match_telemetry(definition: &'static str) -> Contract {
    Contract::schema("match-telemetry.schema.json", definition)
}

fn refused(name: &str, actor: Actor, status: u16, code: &'static str) -> Probe {
    Probe::new(name, actor).expect_with(Expect::status(status).details_code(code))
}

/// The registry, status reads and the status stream.
fn server_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/servers")
            .ok(200, server_intel("ServerIntelList"))
            .no_ownership(SHARED_INTEL)
            .not_applicable(
                Dimension::Malformed,
                "the route reads no path parameter, query or body",
            )
            .not_applicable(
                Dimension::Boundary,
                "no parameter, body or paging: the listing is every server the caller may see",
            ),
        RouteSpec::authenticated("GET /api/v1/servers/{id}/status")
            .ok(200, server_intel("ServerIntel"))
            .no_ownership(SHARED_INTEL)
            .boundary(
                Probe::new("a deactivated server is hidden from members", ENLISTED)
                    .fixture("deactivated-server")
                    .expect(404),
            )
            .boundary(
                Probe::new("an administrator sees a deactivated server", ADMIN)
                    .fixture("deactivated-server"),
            ),
        RouteSpec::authenticated("GET /api/v1/servers/{id}/status/stream")
            .ok(
                200,
                Contract::event_stream("server-intel.schema.json", "ServerStatus"),
            )
            .no_ownership(SHARED_INTEL)
            .boundary(
                Probe::new(
                    "a deactivated server's stream is hidden from members",
                    ENLISTED,
                )
                .fixture("deactivated-server")
                .expect(404),
            ),
        RouteSpec::role("POST /api/v1/servers", Role::Admin)
            .ok(201, server_intel("ServerIntel"))
            .request_contract(server_intel("ServerRegistration"))
            .no_ownership(FLEET_ADMINISTRATION)
            .malformed(
                Probe::new("hostname for ip", ADMIN)
                    .merge_body(json!({"ip": "server.example.com"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("textual port", ADMIN)
                    .merge_body(json!({"port": "2302"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("missing port", ADMIN)
                    .change(Change::RemoveField("port"))
                    .expect(400),
            )
            .boundary(
                Probe::new("port 0", ADMIN)
                    .merge_body(json!({"port": 0}))
                    .expect(400),
            )
            .boundary(
                Probe::new("port 65536", ADMIN)
                    .merge_body(json!({"port": 65536}))
                    .expect(400),
            )
            .boundary(Probe::new("port 65535", ADMIN).merge_body(json!({"port": 65535})))
            .boundary(
                Probe::new("unknown modpack", ADMIN)
                    .merge_body(json!({"required_modpack_id": UNSTORED_ID}))
                    .expect(400),
            ),
        RouteSpec::role("PATCH /api/v1/servers/{id}", Role::Admin)
            .ok(200, server_intel("ServerIntel"))
            .request_contract(server_intel("ServerChange"))
            .no_ownership(FLEET_ADMINISTRATION)
            .malformed(
                Probe::new("empty change", ADMIN)
                    .body(json!({}))
                    .expect(400),
            )
            .malformed(
                Probe::new("hostname for ip", ADMIN)
                    .merge_body(json!({"ip": "server.example.com"}))
                    .expect(400),
            )
            .boundary(
                Probe::new("port 0", ADMIN)
                    .merge_body(json!({"port": 0}))
                    .expect(400),
            )
            .boundary(
                Probe::new("clearing the modpack", ADMIN)
                    .merge_body(json!({"required_modpack_id": null})),
            )
            .boundary(
                Probe::new("deactivating through a change", ADMIN)
                    .merge_body(json!({"is_active": false})),
            ),
        RouteSpec::role("DELETE /api/v1/servers/{id}", Role::Admin)
            .ok(204, Contract::NoBody)
            .no_ownership(FLEET_ADMINISTRATION)
            .boundary(
                Probe::new("deactivating a deactivated server again", ADMIN)
                    .fixture("deactivated-server"),
            ),
    ]
}

/// The administrator side of the fleet command ledger.
fn command_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("POST /api/v1/servers/{id}/commands", Role::Admin)
            .ok(202, fleet_command("FleetCommandReceipt"))
            .decodes(round_trip::<FleetCommandReceipt>)
            .request_contract(fleet_command("FleetCommandRequest"))
            .no_ownership(FLEET_ADMINISTRATION)
            .malformed(
                Probe::new("unknown field", ADMIN)
                    .merge_body(json!({"priority": 1}))
                    .expect(400),
            )
            .malformed(
                Probe::new("unknown action", ADMIN)
                    .merge_body(json!({"action": "self_destruct"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("deployment-only action", ADMIN)
                    .merge_body(json!({"action": "load_mission"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("a console line holding a newline", ADMIN)
                    .body(json!({"action": "console_command",
                        "arguments": {"line": "#players\n#shutdown"}}))
                    .expect(400),
            )
            .malformed(
                Probe::new("a console line starting with @", ADMIN)
                    .body(json!({"action": "console_command", "arguments": {"line": "@logout"}}))
                    .expect(400),
            )
            .boundary(
                Probe::new("console line of 257 bytes", ADMIN)
                    .body(json!({"action": "console_command",
                        "arguments": {"line": "c".repeat(257)}}))
                    .expect(400),
            )
            .boundary(
                Probe::new("console line of 256 bytes", ADMIN).body(
                    json!({"action": "console_command", "arguments": {"line": "c".repeat(256)}}),
                ),
            )
            .boundary(
                Probe::new("broadcast of 257 bytes", ADMIN)
                    .body(json!({"action": "broadcast", "arguments": {"message": "b".repeat(257)}}))
                    .expect(400),
            )
            .boundary(
                Probe::new("broadcast of 256 bytes", ADMIN).body(
                    json!({"action": "broadcast", "arguments": {"message": "b".repeat(256)}}),
                ),
            )
            .boundary(
                Probe::new("a deactivated server accepts no commands", ADMIN)
                    .fixture("deactivated-server")
                    .body(json!({"action": "list_players"}))
                    .expect(409),
            ),
        RouteSpec::role("GET /api/v1/servers/{id}/commands", Role::Admin)
            .ok(200, fleet_command("FleetCommandList"))
            .decodes(round_trip::<FleetCommandList>)
            .no_ownership(FLEET_ADMINISTRATION)
            .malformed(
                Probe::new("non-numeric limit", ADMIN)
                    .query("limit=x")
                    .expect(400),
            )
            .malformed(
                Probe::new("negative offset", ADMIN)
                    .query("offset=-1")
                    .expect(400),
            )
            .boundary(
                Probe::new("limit 100 is the ceiling", ADMIN)
                    .query("limit=100")
                    .expect_with(Expect::success().max_items("/items", 100)),
            )
            .boundary(
                Probe::new("limit 101", ADMIN)
                    .query("limit=101")
                    .expect(400),
            )
            .boundary(Probe::new("limit 0", ADMIN).query("limit=0").expect(400)),
        RouteSpec::role("GET /api/v1/servers/{id}/commands/{commandId}", Role::Admin)
            .ok(200, fleet_command("FleetCommandReceipt"))
            .decodes(round_trip::<FleetCommandReceipt>)
            .no_ownership(FLEET_ADMINISTRATION)
            .boundary(
                Probe::new("a command read through another server", ADMIN)
                    .fixture("command-of-other-server")
                    .expect(404),
            ),
        RouteSpec::role(
            "POST /api/v1/servers/{id}/commands/{commandId}/cancel",
            Role::Admin,
        )
        .ok(200, fleet_command("FleetCommandReceipt"))
        .decodes(round_trip::<FleetCommandReceipt>)
        .no_ownership(FLEET_ADMINISTRATION)
        .boundary(
            Probe::new("cancelling a cancelled command again", ADMIN)
                .fixture("cancelled-command")
                .expect_with(Expect::success().json_at("/state", json!("cancelled"))),
        )
        .boundary(
            refused(
                "a claimed command is no longer cancellable",
                ADMIN,
                409,
                "COMMAND_NOT_CANCELLABLE",
            )
            .fixture("claimed-command"),
        ),
    ]
}

/// The executor side of the fleet command ledger.
fn executor_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::machine(
            "POST /api/v1/fleet-executor/commands/claim",
            Executor::HostAgent,
        )
        .ok(200, fleet_command("ClaimedFleetCommand"))
        .decodes(round_trip::<ClaimedFleetCommand>)
        .request_contract(fleet_command("ClaimRequest"))
        .override_derived(
            "wrong executor",
            Expect::status(204).contract(Contract::NoBody),
            "both executor kinds claim here: a game runtime claims from its own executor \
                 queue within its open session, and that queue holds none of the host agent's \
                 commands",
        )
        .ownership(
            Probe::new(
                "another server's credential claims only its own queue",
                OTHER_HOST_AGENT,
            )
            .expect_with(Expect::status(204).contract(Contract::NoBody)),
        )
        .malformed(
            Probe::new("unknown field", HOST_AGENT)
                .merge_body(json!({"server_id": UNSTORED_ID}))
                .expect(400),
        )
        .malformed(
            Probe::new("a game runtime claims without its session", RUNTIME)
                .body(json!({}))
                .expect(400),
        )
        .boundary(
            Probe::new("nothing claimable", HOST_AGENT)
                .fixture("claim-drained")
                .expect_with(Expect::status(204).contract(Contract::NoBody)),
        ),
        RouteSpec::machine(
            "POST /api/v1/fleet-executor/commands/{commandId}/executing",
            Executor::HostAgent,
        )
        .ok(200, fleet_command("FleetCommandReceipt"))
        .decodes(round_trip::<FleetCommandReceipt>)
        .request_contract(fleet_command("ExecutionStart"))
        .override_derived(
            "wrong executor",
            Expect::status(409).details_code("STALE_FENCING_TOKEN"),
            EXECUTOR_KINDS,
        )
        .ownership(
            Probe::new("another server's credential", OTHER_HOST_AGENT)
                .expect_with(Expect::status(403).error_contains("another server")),
        )
        .malformed(
            Probe::new("unknown field", HOST_AGENT)
                .merge_body(json!({"note": "route acceptance"}))
                .expect(400),
        )
        .malformed(
            Probe::new("textual fencing token", HOST_AGENT)
                .merge_body(json!({"fencing_token": "one"}))
                .expect(400),
        )
        .boundary(
            refused(
                "stale fencing token",
                HOST_AGENT,
                409,
                "STALE_FENCING_TOKEN",
            )
            .fixture("executing-stale-token"),
        )
        .boundary(
            Probe::new("a repeated executing report is idempotent", HOST_AGENT)
                .fixture("executing-command")
                .expect_with(Expect::success().json_at("/state", json!("executing"))),
        ),
        RouteSpec::machine(
            "POST /api/v1/fleet-executor/commands/{commandId}/result",
            Executor::HostAgent,
        )
        .ok(200, fleet_command("FleetCommandReceipt"))
        .decodes(round_trip::<FleetCommandReceipt>)
        .request_contract(fleet_command("ExecutionResult"))
        .override_derived(
            "wrong executor",
            Expect::status(409).details_code("STALE_FENCING_TOKEN"),
            EXECUTOR_KINDS,
        )
        .ownership(
            Probe::new("another server's credential", OTHER_HOST_AGENT)
                .expect_with(Expect::status(403).error_contains("another server")),
        )
        .malformed(
            Probe::new("unknown field", HOST_AGENT)
                .merge_body(json!({"note": "route acceptance"}))
                .expect(400),
        )
        .malformed(
            Probe::new("a failure without its reason", HOST_AGENT)
                .merge_body(json!({"succeeded": false}))
                .expect(400),
        )
        .boundary(
            Probe::new("failure reason of 513 bytes", HOST_AGENT)
                .merge_body(json!({"succeeded": false, "failure_reason": "f".repeat(513)}))
                .expect(400),
        )
        .boundary(
            Probe::new("failure reason of 512 bytes", HOST_AGENT)
                .merge_body(json!({"succeeded": false, "failure_reason": "f".repeat(512)}))
                .expect_with(Expect::success().json_at("/state", json!("failed"))),
        )
        .boundary(
            Probe::new("console response of 4096 bytes", HOST_AGENT)
                .fixture("executing-console-command")
                .merge_body(json!({"succeeded": true,
                    "outcome": {"response": "r".repeat(4096), "response_truncated": true}}))
                .expect_with(Expect::success().json_at("/state", json!("succeeded"))),
        )
        // Last user of its fixture: the refused command stays executing and holds back any
        // later console claim.
        .boundary(
            Probe::new("console response of 4097 bytes", HOST_AGENT)
                .fixture("executing-console-command")
                .merge_body(json!({"succeeded": true,
                    "outcome": {"response": "r".repeat(4097), "response_truncated": true}}))
                .expect(400),
        )
        .boundary(
            refused(
                "success before an executing report",
                HOST_AGENT,
                409,
                "COMMAND_NOT_EXECUTING",
            )
            .fixture("result-before-executing"),
        ),
    ]
}

/// Machine credentials of a server.
fn credential_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("GET /api/v1/servers/{id}/credentials", Role::Admin)
            .ok(200, machine_credential("MachineCredentialList"))
            .decodes(round_trip::<MachineCredentialList>)
            .no_ownership(FLEET_ADMINISTRATION),
        RouteSpec::role("POST /api/v1/servers/{id}/credentials", Role::Admin)
            .ok(201, Contract::schema_root("machine-credential.schema.json"))
            .decodes(round_trip::<IssuedMachineCredential>)
            .request_contract(machine_credential("MachineCredentialIssue"))
            .no_ownership(FLEET_ADMINISTRATION)
            .malformed(
                Probe::new("unknown executor kind", ADMIN)
                    .merge_body(json!({"executor_kind": "operator"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("missing label", ADMIN)
                    .change(Change::RemoveField("label"))
                    .expect(400),
            )
            .malformed(
                Probe::new("blank label", ADMIN)
                    .merge_body(json!({"label": "   "}))
                    .expect(400),
            )
            .boundary(
                Probe::new("label of 129 bytes", ADMIN)
                    .merge_body(json!({"label": "l".repeat(129)}))
                    .expect(400),
            )
            .boundary(
                Probe::new("label of 128 bytes", ADMIN)
                    .merge_body(json!({"label": "l".repeat(128)})),
            )
            .boundary(
                Probe::new("a deactivated server receives no credentials", ADMIN)
                    .fixture("deactivated-server")
                    .body(json!({"executor_kind": "host_agent", "label": "Route acceptance"}))
                    .expect(409),
            ),
        RouteSpec::role(
            "DELETE /api/v1/servers/{id}/credentials/{credentialId}",
            Role::Admin,
        )
        .ok(200, machine_credential("MachineCredential"))
        .decodes(round_trip::<MachineCredential>)
        .no_ownership(FLEET_ADMINISTRATION)
        .malformed(
            Probe::new("missing reason", ADMIN)
                .query("note=route-acceptance")
                .expect(400),
        )
        .malformed(
            Probe::new("blank reason", ADMIN)
                .query("reason=%20%20")
                .expect(400),
        )
        .boundary(
            Probe::new("reason of 513 bytes", ADMIN)
                .query(format!("reason={}", "r".repeat(513)))
                .expect(400),
        )
        .boundary(
            Probe::new("revoking a revoked credential again", ADMIN).fixture("revoked-credential"),
        ),
    ]
}

/// Game-runtime sessions and their heartbeats.
fn runtime_session_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::machine("POST /api/v1/game-runtime/sessions", Executor::ModRuntime)
            .ok(
                201,
                Contract::schema_root("game-runtime-session.schema.json"),
            )
            .decodes(round_trip::<StartedRuntimeSession>)
            .request_contract(runtime_session("RuntimeSessionStart"))
            .no_ownership(SESSION_OF_CALLER)
            .malformed(
                Probe::new("unknown field", RUNTIME)
                    .merge_body(json!({"server_id": UNSTORED_ID}))
                    .expect(400),
            )
            .malformed(
                Probe::new("artifact id without its digest", RUNTIME)
                    .body(json!({"loaded_artifact_id": UNSTORED_ID}))
                    .expect(400),
            )
            .boundary(
                Probe::new(
                    "an empty body starts a session running no artifact",
                    RUNTIME,
                )
                .change(Change::RawBody(Vec::new(), None)),
            )
            .boundary(
                refused("unknown artifact", RUNTIME, 422, "UNKNOWN_ARTIFACT").body(json!({
                    "loaded_artifact_id": UNSTORED_ID,
                    "loaded_artifact_sha256": "0".repeat(64),
                })),
            ),
        RouteSpec::machine(
            "POST /api/v1/game-runtime/sessions/{sessionId}/end",
            Executor::ModRuntime,
        )
        .ok(200, runtime_session("RuntimeSessionEnd"))
        .decodes(round_trip::<RuntimeSessionEnd>)
        .ownership(
            Probe::new("another server's credential", OTHER_RUNTIME)
                .expect_with(Expect::status(403).error_contains("another server")),
        )
        .boundary(
            Probe::new("ending an ended session reports how it ended", RUNTIME)
                .fixture("ended-session")
                .expect_with(Expect::success().json_at("/end_reason", json!("ended_by_runtime"))),
        )
        .boundary(
            Probe::new("a superseded session reports its supersession", RUNTIME)
                .fixture("superseded-session")
                .expect_with(Expect::success().json_at("/end_reason", json!("superseded"))),
        ),
        RouteSpec::machine(
            "POST /api/v1/game-runtime/sessions/{sessionId}/heartbeats",
            Executor::ModRuntime,
        )
        .ok(
            200,
            Contract::schema("runtime-heartbeat-receipt.schema.json", "HeartbeatReceipt"),
        )
        .request_contract(runtime_session("RuntimeHeartbeat"))
        .ownership(
            Probe::new("another server's credential", OTHER_RUNTIME)
                .expect_with(Expect::status(403).error_contains("another server")),
        )
        .malformed(
            Probe::new("unknown field", RUNTIME)
                .merge_body(json!({"note": "route acceptance"}))
                .expect(400),
        )
        .malformed(
            Probe::new("server id in the body", RUNTIME)
                .merge_body(json!({"server_id": UNSTORED_ID}))
                .expect(400),
        )
        .malformed(
            Probe::new("a fence without a reading", RUNTIME)
                .fixture("heartbeat-fence-only")
                .expect(400),
        )
        .boundary(
            refused("stale generation", RUNTIME, 409, "STALE_GENERATION")
                .fixture("heartbeat-stale-generation"),
        )
        .boundary(
            refused("a repeated sequence", RUNTIME, 409, "STALE_SEQUENCE")
                .fixture("heartbeat-replayed"),
        )
        .boundary(
            Probe::new("queue backlog over its capacity", RUNTIME)
                .merge_body(json!({"telemetry_queue": {
                    "backlog": 5, "capacity": 4, "dropped_total": 0, "oldest_age_seconds": 0
                }}))
                .expect(400),
        ),
    ]
}

/// Match telemetry ingest and the match event read.
fn telemetry_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::machine("POST /api/v1/ingest/matches", Executor::ModRuntime)
            .ok(201, match_telemetry("MatchRegistrationAnswer"))
            .decodes(round_trip::<MatchRegistrationAnswer>)
            .request_contract(match_telemetry("MatchRegistration"))
            .ownership(
                Probe::new(
                    "another server's credential names this server's session",
                    OTHER_RUNTIME,
                )
                .expect_with(Expect::status(403).error_contains("another server")),
            )
            .malformed(
                Probe::new("server id in the body", RUNTIME)
                    .merge_body(json!({"server_id": UNSTORED_ID}))
                    .expect(400),
            )
            .malformed(
                Probe::new("unparseable start", RUNTIME)
                    .merge_body(json!({"started_at": "yesterday"}))
                    .expect(400),
            )
            .boundary(
                Probe::new("source match id of 129 bytes", RUNTIME)
                    .merge_body(json!({"source_match_id": "m".repeat(129)}))
                    .expect(400),
            )
            .boundary(
                Probe::new("unknown runtime session", RUNTIME)
                    .merge_body(json!({"runtime_session_id": UNSTORED_ID}))
                    .expect(400),
            )
            .boundary(
                Probe::new("a repeated registration is inert", RUNTIME)
                    .fixture("registration-repeated")
                    .expect_with(
                        Expect::status(200)
                            .contract(match_telemetry("MatchRegistrationAnswer"))
                            .json_at("/registered", json!(false)),
                    ),
            )
            .boundary(
                refused(
                    "another registration of a registered source",
                    RUNTIME,
                    409,
                    "REGISTRATION_CONFLICT",
                )
                .fixture("registration-repeated")
                .merge_body(json!({"started_at": "2026-01-02T00:00:00Z"})),
            ),
        RouteSpec::machine("POST /api/v1/ingest/match-results", Executor::ModRuntime)
            .ok(200, match_telemetry("MatchResultsAnswer"))
            .decodes(round_trip::<MatchResultsAnswer>)
            .request_contract(match_telemetry("MatchResultsRevision"))
            .ownership(refused(
                "another server never reaches this server's match",
                OTHER_RUNTIME,
                409,
                "MATCH_NOT_REGISTERED",
            ))
            .malformed(
                Probe::new("unknown field", RUNTIME)
                    .merge_body(json!({"server_id": UNSTORED_ID}))
                    .expect(400),
            )
            .malformed(
                Probe::new("revision 0", RUNTIME)
                    .merge_body(json!({"revision": 0}))
                    .expect(400),
            )
            .boundary(
                refused("unregistered match", RUNTIME, 409, "MATCH_NOT_REGISTERED")
                    .fixture("results-unregistered"),
            )
            .boundary(
                Probe::new("the same revision again is inert", RUNTIME)
                    .fixture("results-applied")
                    .expect_with(Expect::success().json_at("/applied", json!(false))),
            )
            .boundary(
                refused(
                    "the same revision with another digest",
                    RUNTIME,
                    409,
                    "REVISION_CONFLICT",
                )
                .fixture("results-applied")
                .merge_body(json!({"players": []})),
            )
            .boundary(
                refused("an older revision", RUNTIME, 409, "STALE_REVISION")
                    .fixture("results-superseded"),
            ),
        RouteSpec::machine("POST /api/v1/ingest/match-events", Executor::ModRuntime)
            .ok(200, match_telemetry("MatchEventBatchAnswer"))
            .decodes(round_trip::<MatchEventBatchAnswer>)
            .request_contract(match_telemetry("MatchEventBatch"))
            .ownership(refused(
                "another server never reaches this server's match",
                OTHER_RUNTIME,
                409,
                "MATCH_NOT_REGISTERED",
            ))
            .malformed(
                Probe::new("unknown field", RUNTIME)
                    .merge_body(json!({"server_id": UNSTORED_ID}))
                    .expect(400),
            )
            .malformed(
                refused("unknown event kind", RUNTIME, 400, "INVALID_EVENT").merge_body(json!({
                    "events": [{
                        "event_id": "route-acceptance-unknown", "sequence": 1,
                        "kind": "combat.surrender", "mission_time_ms": 0,
                        "occurred_at": "2026-01-01T00:00:01Z", "payload": {}
                    }]
                })),
            )
            .boundary(
                Probe::new("an empty batch", RUNTIME)
                    .merge_body(json!({"events": []}))
                    .expect(400),
            )
            .boundary(
                refused(
                    "a batch of 501 events",
                    RUNTIME,
                    400,
                    "EVENT_BATCH_TOO_LARGE",
                )
                .fixture("events-oversized-batch"),
            )
            .boundary(
                Probe::new("a stored batch again counts as duplicates", RUNTIME)
                    .fixture("events-stored")
                    .expect_with(
                        Expect::success()
                            .json_at("/accepted", json!(0))
                            .json_at("/duplicates", json!(1)),
                    ),
            )
            .boundary(
                refused("unregistered match", RUNTIME, 409, "MATCH_NOT_REGISTERED")
                    .merge_body(json!({"source_match_id": "route-acceptance-unregistered"})),
            ),
        RouteSpec::authenticated("GET /api/v1/matches/{matchId}/events")
            .ok(200, Contract::schema_root("match-telemetry.schema.json"))
            .decodes(round_trip::<MatchEventPage>)
            .no_ownership("match events are shared telemetry; every signed-in member reads them")
            .malformed(
                Probe::new("non-numeric limit", ENLISTED)
                    .query("limit=x")
                    .expect(400),
            )
            .malformed(
                Probe::new("negative after_sequence", ENLISTED)
                    .query("after_sequence=-1")
                    .expect(400),
            )
            .boundary(
                Probe::new("limit 501", ENLISTED)
                    .query("limit=501")
                    .expect(400),
            )
            .boundary(
                Probe::new("limit 1 pages the events", ENLISTED)
                    .query("limit=1")
                    .expect_with(
                        Expect::success()
                            .max_items("/items", 1)
                            .json_at("/next_after_sequence", json!(1)),
                    ),
            )
            .boundary(
                Probe::new("the last page names no next sequence", ENLISTED)
                    .query("after_sequence=1&limit=500")
                    .expect_with(
                        Expect::success()
                            .max_items("/items", 1)
                            .json_at("/next_after_sequence", json!(null)),
                    ),
            ),
    ]
}

/// Every spec of the `fleet_and_telemetry` part.
pub fn specs() -> Vec<RouteSpec> {
    [
        server_specs(),
        command_specs(),
        executor_specs(),
        credential_specs(),
        runtime_session_specs(),
        telemetry_specs(),
    ]
    .concat()
}
