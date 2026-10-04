# Server infrastructure domain

The [API](/documentation/glossary/a_to_f.md#api)'s
[server infrastructure](/documentation/glossary/n_to_z.md#server-infrastructure) domain: the game server
fleet. It holds the [registry](/documentation/glossary/n_to_z.md#registry) row that describes a server
and the modpack it requires, the per-server
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential) of its host agent and
[game runtime](/documentation/glossary/g_to_m.md#game-runtime), the runtime sessions that fence each
boot of the game runtime, the [fleet command](/documentation/glossary/a_to_f.md#fleet-command) ledger
through which operators control servers and executors report what they did, the
[fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario) registry, the server intel reads, and
the [SSE](/documentation/glossary/n_to_z.md#sse) feed of one server's live status.

## Contents

```text
crates/api/api_server_infrastructure/src/
├── error.rs    the crate's error type and `Result` alias, converting into `ApiError`
├── handlers/   the intel, registry, credential, command, executor, session, scenario and stream handlers
├── lib.rs      the crate root: the module tree; re-exports `routes`, `Error` and `Result`
├── models/     the server, status, fleet command, credential and fleet scenario shapes
├── prelude.rs  the status, publisher, heartbeat fence and reconciliation names other crates import
├── routes.rs   the domain's `/api/v1` route table
└── services/   machine credentials, runtime sessions, the fleet command ledger, the status publisher
```

## How it works

Server registration, identity and presentation live here. The heartbeat that writes a server's
live status, the game runtime's telemetry queue reading included, arrives through
`api_match_telemetry`, which fences it with this domain's runtime sessions and publishes the stored row
with this domain's status publisher. `ServerStatus.telemetry_queue` carries that reading through the
status read, the stream and every publish, and is absent for a server that never reported one.

The configured fleet is the set of servers with `is_active = true`. Members see only the fleet:
`GET /servers` lists active servers to everyone and every server, with `is_active`, to
administrators, and the status read and the status stream of an inactive server are a 404 for
anyone but an administrator. The scheduled publisher republishes active servers only. The realtime hub itself is
`api_http_layer::realtime_hub`; the queries and publishers that feed the `server:{id}` topic are this
domain's, so the API's router and the API crates name no server concept.

Operators never reach a game host directly. An administrator's command becomes a durable row in
the ledger, and the program that performs it polls the API outbound with its own machine
credential: the [fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent) runs process
control, the [RCON](/documentation/glossary/n_to_z.md#rcon) player list and console commands, and the
game runtime runs broadcasts, kicks and in-process [mission](/documentation/glossary/g_to_m.md#mission)
loads. A [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) in `api_missions`
issues its `load_mission` or `restart_with_mission` command through the same ledger and needs a
fleet scenario for its terrain. A console command (`console_command`) is the one free-text action:
a single line for the server's RCON console, gated to 1 to 256 bytes, one line, no control
characters and no leading `@`; the host agent transmits it at most once and reports the reply,
bounded at 4096 bytes. It changes the server process as a restart does, so it waits for and holds
back the server's other process changes and is never repeated after an unknown outcome. There is
no separate RCON route: the console line travels through the command ledger like every other
action.

## Public surface

- `routes::routes()`: the table the API's router (`api::router`) merges under `/api/v1`, one route each:
  - `GET` and `POST /api/v1/servers`: `AuthUser` to list the intel cards (active servers; every
    server for an administrator), `AdminUser` to register.
  - `PATCH` and `DELETE /api/v1/servers/{id}`: `AdminUser`; partial update, deactivate.
  - `GET /api/v1/servers/{id}/status`: `AuthUser`; one server's intel card (404 for an inactive
    server unless the caller is an administrator).
  - `GET /api/v1/servers/{id}/status/stream`: `AuthUser`; the live status feed, scoped the same
    way.
  - `GET` and `POST /api/v1/servers/{id}/credentials`: `AdminUser`; list, issue (secret once).
  - `DELETE /api/v1/servers/{id}/credentials/{credentialId}`: `AdminUser`; revoke with a reason.
  - `GET` and `POST /api/v1/servers/{id}/commands`: `AdminUser`; receipts, request (202).
  - `GET /api/v1/servers/{id}/commands/{commandId}`: `AdminUser`; one receipt.
  - `POST /api/v1/servers/{id}/commands/{commandId}/cancel`: `AdminUser`; cancel while unclaimed.
  - `POST /api/v1/fleet-executor/commands/claim`: machine credential; claim the next command.
  - `POST /api/v1/fleet-executor/commands/{commandId}/executing`: machine credential; the start.
  - `POST /api/v1/fleet-executor/commands/{commandId}/result`: machine credential; the outcome.
  - `POST /api/v1/game-runtime/sessions`: `mod_runtime` credential; start the next generation.
  - `POST /api/v1/game-runtime/sessions/{sessionId}/end`: `mod_runtime` credential; end it.
  - `GET /api/v1/fleet/scenarios`: `AdminUser`; the scenario of every terrain.
  - `PUT` and `DELETE /api/v1/fleet/scenarios/{terrainKey}`: `AdminUser`; register or replace,
    withdraw.
- `services::runtime_sessions`: the heartbeat fence for `api_match_telemetry`, the open-session share
  lock for live [slot](/documentation/glossary/n_to_z.md#slot) occupancy in `api_operations`, and silence
  expiry for its worker.
- `services::status_broadcast`: `publish_server_status`, `publish_server_status_by_id` and
  `publish_all_server_statuses`, for the heartbeat and the status workers, and
  `SELECT_FLEET_STATUSES`, the configured fleet's status rows the dashboard reads.
- `services::fleet_commands`: the command ledger mission deployments issue through, and
  `reconcile_fleet_commands` for its worker.
- `models`: `Server`, `ServerStatus` with its `TelemetryQueueStatus`, and the `ServerStatusRow`
  projection, read by the dashboard and the heartbeat. The domain's generated contract types
  (`contract_schema_types::server_infrastructure`) are read by the contract test
  `apps/api/tests/game_runtime_contract.rs`. `ExecutorKind` and `FleetAction`, which
  `api_match_telemetry`, `api_missions` and `api_operations` read, live in the `fleet_wire_contract` crate.

## Boundaries

- Depends on:
  - `api_state` for the application state; `api_foundation` for the handler error and the path
    parameters; `api_http_layer` for the extractors, `role_rank`, the event stream authorization,
    the token primitives and the realtime hub; `api_caller_identity` for the machine caller, the
    status stream's session authorization, the administrator recheck inside a transaction and the
    account locks; `api_audit_log` for the audit rows of every registry, credential, command,
    session and scenario write; `api_failpoints` for the command claim and result failpoints;
    `api_identifiers` for the typed ids;
  - `fleet_wire_contract` for the fleet command wire shapes, `FleetAction`, `ExecutorKind`, the
    machine credential prefix and the secret-file limits it shares with the fleet host agent;
  - `api_community_content` for the modpack a server requires (`modpack_lookup`), and
    `api_mission_vocabulary` for the terrain a server runs. It names no other domain.
- Used by:
  - the API's router (`apps/api/src/router.rs`), which merges the route table, and the
    `server_status_publisher`, `runtime_session_expiry` and `fleet_command_reconciler` workers in
    `crates/api/api_background_workers/src/`;
  - `api_match_telemetry`, `api_missions`, `api_operations` and `api_command_center`, through the surface above,
    and the `staging-fixtures` host tool in `tools/staging/staging_fixtures/`;
  - over HTTP, the [server control](/documentation/glossary/n_to_z.md#server-control) and server intel
    pages in the page crates under `crates/frontend/pages/`, the fleet host agent in
    `apps/fleet_host_agent/`, and the game runtime in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges (`apps/api/src/tests/architecture_rules.rs` checks both); every handler
  carries its `/// @route` tag (`cargo xtask verify route-tags`); the domain's generated contract
  types are written by `cargo xtask ci schema-codegen` and never edited by hand; a machine acts only for its own server
  and executor kind.

## Related documentation

- [API overview](/documentation/apps/api/api_overview.md) — every domain's routes.
- [API decisions](/documentation/apps/api/decisions.md) — why game hosts are reached only
  through commands they claim.
- [Machine credentials and runtime sessions](/documentation/apps/api/verification_evidence/machine_credentials.md)
  — credentials, the session fence and their consumers.
- [Match telemetry, fleet status and derived statistics](/documentation/apps/api/verification_evidence/telemetry.md)
  — the telemetry queue reading on the status and the configured fleet's scoping.
- [Fleet command ledger](/documentation/apps/api/verification_evidence/fleet_command_ledger.md)
  — the ledger's commands, states, rules and executors.
- [Live slot occupancy](/documentation/apps/api/verification_evidence/live_occupancy.md)
  — how player lives hold a runtime session open.
- [Server control page](/documentation/crates/frontend/pages/administration_pages/server_control/server_control_page.md)
  — the administrators' console over these routes.
