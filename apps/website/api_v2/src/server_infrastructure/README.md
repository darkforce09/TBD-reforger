# `server_infrastructure/`

The dedicated-server fleet: the registry row that describes a server and the modpack it runs, the
Server Intel reads, the Server-Sent Events feed carrying one server's live status, and the admin
RCON console that delivers an operator command to the game host's control agent.

Server *registration and presentation* live here; the heartbeat that writes a live status row comes
in through `match_telemetry`'s ingest endpoint, and this domain publishes the result onto the SSE
topic.

## Public surface

- **`routes::routes()`** — the domain's `/api/v1` table, merged by
  `core::http_router::api_v1_routes` and nested under `/api/v1`. The literals in `routes.rs` are the
  public URLs: `/servers`, `/servers/{id}`, `/servers/{id}/status`, `/servers/{id}/status/stream`,
  `/admin/servers/{id}/rcon`.
- **`services::status_broadcast`** — `publish_server_status` (one server) and
  `publish_all_server_statuses` (the scheduled republish the background worker calls). Both write to
  the `server:{id}` topic on `core::realtime_hub`.
- **`services::game_agent`** — the client for the game host's control agent, and the `AgentAction`
  vocabulary the RCON console speaks.
- **`models::server`** — `Server` (the registration), `ServerStatus` (the single hot status row),
  and the time series those status rows are archived into.

## Dependency rules

- Handlers here never import another domain's handlers; `src/tests/architecture_rules.rs` enforces
  it across all eight domains.
- This domain imports `core`, `community_content::{models, services}` (the modpack a server is bound
  to, resolved through `modpack_lookup`), `missions::models` (the mission a server is staged with)
  and `administration::{models, services}` (the papertrail an RCON command and a registry write
  leave).
- Nothing in `core` imports it. The SSE topic layer itself is `core::realtime_hub`; the queries and
  publishers that feed it are this domain's, so `core` names no server concept.

## Files

```text
mod.rs                                 Domain module tree; re-exports `routes`.
routes.rs                              The `/api/v1` route table for server infrastructure.
handlers/
  mod.rs                               The intel reads, registry writes, live status stream, and RCON console.
  rcon_command_parser.rs               The RCON request boundary: the wire body and the command it parses into.
  rcon_console.rs                      The admin RCON console: deliver a validated command to the host agent.
  server_intel.rs                      Server Intel reads: the `GET` half of the server surface.
  server_registry.rs                   Registry writes: create, partially update, and deactivate a `servers` row.
  server_status_stream.rs              The Server-Sent Events feed for one server's live status.
  tests/
    rcon_command_parser.rs             Sibling unit tests for `rcon_command_parser.rs`.
    rcon_console.rs                    Sibling unit tests for `rcon_console.rs`.
    server_intel.rs                    Sibling unit tests for `server_intel.rs`.
services/
  mod.rs                               The host-agent client and the status publishers.
  game_agent.rs                        Client for the game host's control agent.
  status_broadcast.rs                  The `server:{id}` SSE topic: the live-status query and its publishers.
  tests/
    game_agent.rs                      Sibling unit tests for `game_agent.rs`.
    status_broadcast.rs                Sibling unit tests for `status_broadcast.rs`.
models/
  mod.rs                               Database and wire models for the dedicated-server fleet.
  server.rs                            Registration, the single hot status row, and the status time series.
```

Unit tests live in the sibling files above, declared from the production file as
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
