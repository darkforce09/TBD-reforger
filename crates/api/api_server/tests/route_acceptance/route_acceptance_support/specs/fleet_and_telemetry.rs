//! Route specs of the `fleet_and_telemetry` part: the server registry and its live status,
//! machine credentials, the fleet command ledger and its executor routes, game-runtime
//! sessions and heartbeats, and the match telemetry ingest and read routes.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the authorized caller and fixture where the defaults do not fit, and the unauthorized probes
//! and overrides the access class does not imply.

use super::super::spec::{Contract, Executor, Expect, Role, RouteSpec};

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

/// The registry, status reads and the status stream.
fn server_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/servers").ok(200, server_intel("ServerIntelList")),
        RouteSpec::authenticated("GET /api/v1/servers/{id}/status")
            .ok(200, server_intel("ServerIntel")),
        RouteSpec::authenticated("GET /api/v1/servers/{id}/status/stream").ok(
            200,
            Contract::event_stream("server-intel.schema.json", "ServerStatus"),
        ),
        RouteSpec::role("POST /api/v1/servers", Role::Admin).ok(201, server_intel("ServerIntel")),
        RouteSpec::role("PATCH /api/v1/servers/{id}", Role::Admin)
            .ok(200, server_intel("ServerIntel")),
        RouteSpec::role("DELETE /api/v1/servers/{id}", Role::Admin).ok(204, Contract::NoBody),
    ]
}

/// The administrator side of the fleet command ledger.
fn command_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("POST /api/v1/servers/{id}/commands", Role::Admin)
            .ok(202, fleet_command("FleetCommandReceipt")),
        RouteSpec::role("GET /api/v1/servers/{id}/commands", Role::Admin)
            .ok(200, fleet_command("FleetCommandList")),
        RouteSpec::role("GET /api/v1/servers/{id}/commands/{commandId}", Role::Admin)
            .ok(200, fleet_command("FleetCommandReceipt")),
        RouteSpec::role(
            "POST /api/v1/servers/{id}/commands/{commandId}/cancel",
            Role::Admin,
        )
        .ok(200, fleet_command("FleetCommandReceipt")),
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
        // Both executor kinds claim here: a game runtime claims from its own executor queue
        // within its open session, and that queue holds none of the host agent's commands.
        .override_derived(
            "wrong executor",
            Expect::status(204).contract(Contract::NoBody),
        ),
        RouteSpec::machine(
            "POST /api/v1/fleet-executor/commands/{commandId}/executing",
            Executor::HostAgent,
        )
        .ok(200, fleet_command("FleetCommandReceipt"))
        // Both executor kinds report on the executor routes; a claim belongs to the credential
        // that took it, so another credential's report is a stale fencing token.
        .override_derived(
            "wrong executor",
            Expect::status(409).details_code("STALE_FENCING_TOKEN"),
        ),
        RouteSpec::machine(
            "POST /api/v1/fleet-executor/commands/{commandId}/result",
            Executor::HostAgent,
        )
        .ok(200, fleet_command("FleetCommandReceipt"))
        // Both executor kinds report on the executor routes; a claim belongs to the credential
        // that took it, so another credential's report is a stale fencing token.
        .override_derived(
            "wrong executor",
            Expect::status(409).details_code("STALE_FENCING_TOKEN"),
        ),
    ]
}

/// Machine credentials of a server.
fn credential_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("GET /api/v1/servers/{id}/credentials", Role::Admin)
            .ok(200, machine_credential("MachineCredentialList")),
        RouteSpec::role("POST /api/v1/servers/{id}/credentials", Role::Admin)
            .ok(201, Contract::schema_root("machine-credential.schema.json")),
        RouteSpec::role(
            "DELETE /api/v1/servers/{id}/credentials/{credentialId}",
            Role::Admin,
        )
        .ok(200, machine_credential("MachineCredential")),
    ]
}

/// Game-runtime sessions and their heartbeats.
fn runtime_session_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::machine("POST /api/v1/game-runtime/sessions", Executor::ModRuntime).ok(
            201,
            Contract::schema_root("game-runtime-session.schema.json"),
        ),
        RouteSpec::machine(
            "POST /api/v1/game-runtime/sessions/{sessionId}/end",
            Executor::ModRuntime,
        )
        .ok(200, runtime_session("RuntimeSessionEnd")),
        RouteSpec::machine(
            "POST /api/v1/game-runtime/sessions/{sessionId}/heartbeats",
            Executor::ModRuntime,
        )
        .ok(
            200,
            Contract::schema("runtime-heartbeat-receipt.schema.json", "HeartbeatReceipt"),
        ),
    ]
}

/// Match telemetry ingest and the match event read.
fn telemetry_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::machine("POST /api/v1/ingest/matches", Executor::ModRuntime)
            .ok(201, match_telemetry("MatchRegistrationAnswer")),
        RouteSpec::machine("POST /api/v1/ingest/match-results", Executor::ModRuntime)
            .ok(200, match_telemetry("MatchResultsAnswer")),
        RouteSpec::machine("POST /api/v1/ingest/match-events", Executor::ModRuntime)
            .ok(200, match_telemetry("MatchEventBatchAnswer")),
        RouteSpec::authenticated("GET /api/v1/matches/{matchId}/events")
            .ok(200, Contract::schema_root("match-telemetry.schema.json")),
    ]
}

/// Every spec of the `fleet_and_telemetry` part.
pub(crate) fn specs() -> Vec<RouteSpec> {
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
