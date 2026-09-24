# Server infrastructure domain

The API's [server infrastructure](/documentation_v2/glossary.md#server-infrastructure) domain: the
game server fleet. It holds the registry row that describes a server and the modpack it requires,
the per-server [machine credentials](/documentation_v2/glossary.md#machine-credential) of its host
agent and game runtime, the runtime sessions that fence each boot of the game runtime, the
[fleet command](/documentation_v2/glossary.md#fleet-command) ledger through which operators control
servers and executors report what they did, the [fleet scenario](/documentation_v2/glossary.md#fleet-scenario)
registry, the server intel reads, and the [SSE](/documentation_v2/glossary.md#sse) feed of one
server's live status.

## Contents

```text
apps/website/api_v2/src/server_infrastructure/
├── handlers/  the intel, registry, credential, command, executor, session, scenario and stream handlers
├── mod.rs     the module tree; re-exports `routes`
├── models/    the server, status, fleet command, credential and fleet scenario shapes
├── routes.rs  the domain's `/api/v1` route table
└── services/  machine credentials, runtime sessions, the fleet command ledger, the status publisher
```

## How it works

Server registration, identity and presentation live here. The heartbeat that writes a server's
live status arrives through `match_telemetry`, which fences it with this domain's runtime sessions
and publishes the stored row with this domain's status publisher. The realtime hub itself is
`core::realtime_hub`; the queries and publishers that feed the `server:{id}` topic are this
domain's, so `core` names no server concept.

Operators never reach a game host directly. An administrator's command becomes a durable row in
the ledger, and the program that performs it polls the API outbound with its own machine
credential: the [fleet host agent](/documentation_v2/glossary.md#fleet-host-agent) runs process
control and the [RCON](/documentation_v2/glossary.md#rcon) player list, and the game runtime runs
broadcasts, kicks and in-process mission loads. A [mission deployment](/documentation_v2/glossary.md#mission-deployment)
in `missions` issues its `load_mission` or `restart_with_mission` command through the same ledger
and needs a fleet scenario for its terrain. The platform has no RCON console route.

## Public surface

- `routes::routes()`: the table `core::http_router` merges under `/api/v1`, one route each:
  - `GET` and `POST /api/v1/servers`: `AuthUser` to list the intel cards, `AdminUser` to register.
  - `PATCH` and `DELETE /api/v1/servers/{id}`: `AdminUser`; partial update, deactivate.
  - `GET /api/v1/servers/{id}/status`: `AuthUser`; one server's intel card.
  - `GET /api/v1/servers/{id}/status/stream`: `AuthUser`; the live status feed.
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
- `services::machine_credentials::MachineCaller`: the machine caller every `/game-runtime/*` and
  `/fleet-executor/*` handler takes, in this domain, `match_telemetry`, `missions` and `operations`.
- `services::runtime_sessions`: the heartbeat fence for `match_telemetry`, the open-session share
  lock for live slot occupancy in `operations`, and silence expiry for its worker.
- `services::status_broadcast`: `publish_server_status`, `publish_server_status_by_id` and
  `publish_all_server_statuses`, for the heartbeat and the status workers.
- `services::fleet_commands`: the command ledger mission deployments issue through, and
  `reconcile_fleet_commands` for its worker.
- `models`: `Server` and `ServerStatus`, read by the dashboard and the heartbeat; `ExecutorKind` and
  `FleetAction`, read by `match_telemetry`, `missions` and `operations`; `generated/`, read by the
  contract test `apps/website/api_v2/tests/game_runtime_contract.rs`.

## Boundaries

- Depends on:
  - `core`: the application state, errors, extractors, `role_rank`, the authentication
    primitives, the realtime hub and the wire formats;
  - `community_content` for the modpack a server requires (`modpack_lookup`),
    `missions::models` for the terrain a server runs, `administration` for the audit rows of every
    registry, credential, command and scenario write, and `identity_and_access` for rechecking an
    administrator inside a transaction and for the requester's authority when a command is claimed.
- Used by:
  - `core::http_router`, which merges the route table, and the `server_status_publisher`,
    `runtime_session_expiry` and `fleet_command_reconciler` workers in
    `apps/website/api_v2/src/background_workers/`;
  - `match_telemetry`, `missions`, `operations` and `command_center`, through the surface above;
  - over HTTP, the [server control](/documentation_v2/glossary.md#server-control) and server intel
    pages in `apps/website/frontend/src/v2/pages/`, the fleet host agent in
    `apps/fleet_host_agent/`, and the game runtime in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges (`apps/website/api_v2/src/tests/architecture_rules.rs` checks both); every handler
  carries its `/// @route` tag (`cargo xtask verify route-tags`); `models/generated/` is written by
  `cargo xtask ci schema-codegen` and never edited by hand; a machine acts only for its own server
  and executor kind.

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — every domain's routes.
- [Machine credentials and runtime sessions](/documentation_v2/website/api_v2/verification_evidence/machine_credentials.md)
  — credentials, the session fence and their consumers.
- [Fleet command ledger](/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md)
  — the ledger's commands, states, rules and executors.
- [Live slot occupancy](/documentation_v2/website/api_v2/verification_evidence/live_occupancy.md)
  — how player lives hold a runtime session open.
- [Server control page](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md)
  — the administrators' console over these routes.
