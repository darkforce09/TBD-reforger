# API server infrastructure

The `api_server_infrastructure` crate: the [API](/documentation/glossary/a_to_f.md#api)'s
[server infrastructure](/documentation/glossary/n_to_z.md#server-infrastructure) domain. It holds
the game server fleet: the [registry](/documentation/glossary/n_to_z.md#registry) row of each
dedicated server, its live status and the `server:{id}` feed that streams it, the per-server
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential), the
[fleet command](/documentation/glossary/a_to_f.md#fleet-command) ledger with its executor claims
and reconciliation, the runtime sessions that fence each boot of the
[game runtime](/documentation/glossary/g_to_m.md#game-runtime), and the
[fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario) of each terrain, with the
`/api/v1` route table the API's router merges.

## Contents

```text
crates/api/api_server_infrastructure/
├── Cargo.toml  the package: `api_state`, `api_caller_identity`, `api_community_content`, `api_http_layer`, `api_audit_log`, `api_failpoints`, `fleet_wire_contract`, sqlx (`postgres`), axum, layout tier 7
└── src/        the route table, the handlers, the credential, session, command ledger and status services, the models, the error and the prelude
```

## How it works

Operators never reach a game host directly. An administrator's command becomes a durable row in
the ledger, and the program that performs it, the
[fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent) or the game runtime, polls
the API outbound with its own machine credential, claims the command under a lease and reports
its start and its outcome. A reconciliation pass returns a command whose lease lapsed to the
queue, expires one nobody completed, and leaves a non-idempotent command whose executor went
silent mid-effect for an operator.

A game runtime starts a session with its `mod_runtime` credential; every heartbeat the match
telemetry domain admits carries that session's generation and a strictly increasing sequence,
so a stale runtime cannot overwrite newer state. The status row each heartbeat writes is published
on the realtime hub's `server:{id}` topic in one serialised shape, in-request and again by the
scheduled publisher. The source tree README has the detail.

## Getting started

Run from the repository root:

```bash
cargo test -p api_server_infrastructure
cargo clippy -p api_server_infrastructure --all-targets -- -D warnings
cargo xtask db test-it --test fleet_command_ledger --test fleet_dashboard --test fleet_console_command --test route_acceptance_fleet_and_telemetry
```

The unit tests cover the command argument and outcome validation, the machine credential secret
format, the status payload shape, the fleet scenario keys and the server intel SQL; the ledger,
the sessions and the routes are proved against Postgres by the API's integration suites.

## Configuration

No feature and no variable of its own. The scheduled status publisher's interval
(`SERVER_STATUS_PUBLISH_INTERVAL_SECS`) belongs to the API's background workers;
[API environment variables](/documentation/apps/api/environment_variables.md) lists it. The
`fail_point!` sites at the command claim and result commits compile to nothing outside test builds
(`api_failpoints`).

## Public surface

- `routes()`: the domain's `/api/v1` route table.
- `handlers`: the server intel, registry, credential, command, executor, runtime session, fleet
  scenario and status stream handlers, each with its `/// @route` tag.
- `services`: the machine credentials, the runtime sessions (the heartbeat fence, the open-session
  share lock, silence expiry), the fleet command ledger, claims, outcomes and reconciliation, the
  server registration and the status publishers with `SELECT_FLEET_STATUSES`.
- `models`: `Server`, `ServerStatus`, `TelemetryQueueStatus`, `ServerStatusRow`,
  `FleetCommandState`, `MachineCredential` and `FleetScenario` with their request and list shapes.
- `Error` and `Result` (a database failure, converting into `ApiError`), and `prelude`.

## Boundaries

- Depends on: `api_state`, `api_caller_identity`, `api_community_content` (the modpack a server
  requires), `api_http_layer`, `api_audit_log`, `api_failpoints`, `api_foundation`,
  `api_identifiers`, `api_mission_vocabulary`, `fleet_wire_contract`, sqlx, axum, async-stream,
  serde, chrono, tokio, thiserror and uuid. It names no other domain.
- Used by: the API application (`apps/api`): its router merges `routes`, its background workers
  run the status publisher, the session expiry and the command reconciliation, the match
  telemetry, missions, operations and command center domains call the services, and the
  `staging-fixtures` host tool and the integration suites reach the services directly. Over HTTP:
  the fleet host agent, the game runtime and the server control and server intel pages.
- Rules: the API crate rules of [crates/api](/crates/api/README.md);
  `apps/api/src/tests/architecture_rules.rs` checks its route table, its handlers and its
  imports against the domain graph.

## Related documentation

- [API server infrastructure source](/crates/api/api_server_infrastructure/src/README.md) — the
  files, the routes and how the fleet is controlled.
- [API overview](/documentation/apps/api/api_overview.md) — every domain's routes.
- [Fleet command ledger](/documentation/apps/api/verification_evidence/fleet_command_ledger.md)
  — the ledger's commands, states, rules and executors.
- [Machine credentials and runtime sessions](/documentation/apps/api/verification_evidence/machine_credentials.md)
  — credentials, the session fence and their consumers.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
