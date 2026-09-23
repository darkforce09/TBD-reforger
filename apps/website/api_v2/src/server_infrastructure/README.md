# `server_infrastructure/`

The dedicated-server fleet: the registry row that describes a server and the modpack it runs, the
per-server machine credentials that authenticate its host agent and game runtime, the runtime
sessions that fence each boot of the game runtime, the fleet command ledger through which
operators control servers and executors report what they did, the Server Intel reads, and the
Server-Sent Events feed carrying one server's live status.

Server *registration, identity and presentation* live here; the heartbeat that writes a live status
row comes in through `match_telemetry`, which fences it with this domain's runtime sessions and
publishes the result onto the SSE topic.

## Public surface

- **`routes::routes()`** — the domain's `/api/v1` table, merged by
  `core::http_router::api_v1_routes` and nested under `/api/v1`. The literals in `routes.rs` are the
  public URLs: `/servers`, `/servers/{id}`, `/servers/{id}/status`, `/servers/{id}/status/stream`,
  `/servers/{id}/credentials`, `/servers/{id}/credentials/{credentialId}`,
  `/servers/{id}/commands`, `/servers/{id}/commands/{commandId}`,
  `/servers/{id}/commands/{commandId}/cancel`, `/fleet-executor/commands/claim`,
  `/fleet-executor/commands/{commandId}/executing`, `/fleet-executor/commands/{commandId}/result`,
  `/game-runtime/sessions`, `/game-runtime/sessions/{sessionId}/end`.
- **`services::machine_credentials`** — issue, list, revoke and verify credentials; `MachineCaller`
  and its executor and server checks. **`services::machine_authentication`** is the request
  extractor every machine route takes.
- **`services::runtime_sessions`** — session start (superseding the previous generation), the
  heartbeat fence, the session share lock deployments hold, runtime end, credential-revocation end
  and silence expiry; ending a session ends the player lives still open in it.
- **`services::status_broadcast`** — `publish_server_status` (one server) and
  `publish_all_server_statuses` (the scheduled republish the background worker calls). Both write to
  the `server:{id}` topic on `core::realtime_hub`.
- **`services::fleet_commands`** — the durable command ledger: operator requests and receipts,
  executor claims under leases and fencing tokens, and time-based crash reconciliation
  (docs/verification/api_v2/fleet_command_ledger.md).
- **`models::server`** — `Server` (the registration), `ServerStatus` (the single hot status row),
  and the time series those status rows are archived into.

## Dependency rules

- Handlers here never import another domain's handlers; `src/tests/architecture_rules.rs` enforces
  it across all eight domains.
- This domain imports `core`, `community_content::{models, services}` (the modpack a server is bound
  to, resolved through `modpack_lookup`), `missions::models` (the mission a server is staged with),
  `administration::{models, services}` (the papertrail a command, a registry write and a
  credential change leave), and `identity_and_access::{models, services}` (reauthorizing an
  administrator on the transaction that changes a credential or a command, and revalidating a
  requester's authority when an executor claims the command).
- Nothing in `core` imports it. The SSE topic layer itself is `core::realtime_hub`; the queries and
  publishers that feed it are this domain's, so `core` names no server concept.

## Files

```text
mod.rs                                 Domain module tree; re-exports `routes`.
routes.rs                              The `/api/v1` route table for server infrastructure.
handlers/
  mod.rs                               The intel reads, registry writes, credentials, commands, sessions, stream.
  fleet_commands.rs                    Administrator command requests, receipts and cancellation.
  fleet_executor.rs                    Executor claim, effect start and outcome reports (machine credential).
  game_runtime_sessions.rs             Game-runtime session start and end (`mod_runtime` credential).
  machine_credentials.rs               Administrator issue, list and revocation of machine credentials.
  server_intel.rs                      Server Intel reads: the `GET` half of the server surface.
  server_registry.rs                   Registry writes: create, partially update, and deactivate a `servers` row.
  server_status_stream.rs              The Server-Sent Events feed for one server's live status.
  tests/
    server_intel.rs                    Sibling unit tests for `server_intel.rs`.
services/
  mod.rs                               Credentials, runtime sessions, the command ledger and status publishers.
  machine_authentication.rs            The `MachineCaller` request extractor for machine routes.
  machine_credentials.rs               Credential issue, listing, revocation and constant-time verification.
  runtime_sessions.rs                  Generation-fenced runtime sessions, heartbeat admission and expiry.
  fleet_commands/
    mod.rs                             The command ledger modules.
    command_arguments.rs               Typed per-action argument validation.
    command_ledger.rs                  Requests, cancellation and receipts.
    command_reconciliation.rs          Expiry, lease lapse and indeterminate outcomes by time.
    executor_claims.rs                 Claims under leases and fencing tokens, effect start and outcomes.
    tests/command_arguments.rs         Sibling unit tests for `command_arguments.rs`.
  status_broadcast.rs                  The `server:{id}` SSE topic: the live-status query and its publishers.
  tests/
    machine_credentials.rs             Sibling unit tests for `machine_credentials.rs`.
    status_broadcast.rs                Sibling unit tests for `status_broadcast.rs`.
models/
  mod.rs                               Database and wire models for the dedicated-server fleet.
  fleet_command.rs                     Command actions, states, receipts, claims and executor reports.
  machine_credential.rs                Executor kinds and the credential views administrators see.
  server.rs                            Registration, the single hot status row, and the status time series.
  generated/                           Types generated from `contracts_v2/definitions` for contract tests.
```

Unit tests live in the sibling files above, declared from the production file as
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
