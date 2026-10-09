# Server infrastructure handlers

The HTTP handlers of the game server fleet: the server intel reads and the
[registry](/documentation/glossary/n_to_z.md#registry) writes,
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential), the
[fleet command](/documentation/glossary/a_to_f.md#fleet-command) ledger for operators and for executors,
runtime sessions, the [fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario) registry and
the live status stream.

## Contents

```text
crates/api/api_server_infrastructure/src/handlers/
├── fleet_commands.rs              administrator command requests, receipts and cancellation
├── fleet_executor.rs              executor claims, start reports and outcome reports, by machine credential
├── fleet_scenarios.rs             the registry of the mission header the fleet boots for each terrain
├── game_runtime_sessions.rs       a game runtime starts and ends its session (`mod_runtime` credential)
├── machine_credentials.rs         administrator issue, list and revocation of machine credentials
├── mod.rs                         the module tree
├── server_administration_lock.rs  locks a server row and rechecks the administrator before a credential or command write
├── server_intel.rs                the server intel reads: each server with its status, modpack and terrain
├── server_registry.rs             create, partially update and deactivate a `servers` row
└── server_status_stream.rs        the SSE feed of one server's live status
```

## How it works

Three tiers meet here. Members read the server intel and the status stream with `AuthUser`.
Administrators (`AdminUser`) write the registry, the credentials, the commands and the fleet
scenarios; every write that changes a credential, a command or a scenario takes its locks, rechecks
the administrator on that transaction (`authorize_on_connection`) and audits inside it. The programs
on a game host present a machine credential (`MachineCaller`): the
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) and the
[game runtime](/documentation/glossary/g_to_m.md#game-runtime) claim and report commands, and the game
runtime starts and ends its runtime session, reporting the
[artifact](/documentation/glossary/a_to_f.md#artifact) it loaded; each acts only for its own server and
executor kind.

- `server_intel.rs` composes one card per server (the registration, its live status row with the
  telemetry queue reading, the modpack it requires and the terrain of the match it runs) for the
  list and the single read alike. The list holds the configured fleet (`is_active = true`) for
  members and every server, marked by `is_active`, for administrators (`sees_inactive_servers`);
  the single read of an inactive server is a 404 for anyone but an administrator.
- `server_registry.rs` validates at the boundary what the `servers` table does not constrain: a
  trimmed, non-blank name, the address, the port and an existing modpack, through
  `services::server_registration`, whose `register_server` writes a new server and its
  `server.create` audit row in one transaction; an explicit `null` for `required_modpack_id`
  clears the modpack, and an absent key keeps it.
- `fleet_commands.rs` answers 202 with the receipt, 404 for the command list of an unknown server
  (as `machine_credentials.rs` does for its credential list), and 400 for `load_mission` and
  `restart_with_mission`, which only a
  [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) issues; `fleet_executor.rs`
  answers 204 when nothing is claimable.
- `fleet_scenarios.rs` keys a scenario by a terrain key (lowercase ASCII, digits and underscores,
  starting with a letter, at most 64 bytes) and a `{16 uppercase hex}` resource ending in `.conf`.
- `server_status_stream.rs` resolves the server before any stream opens: a malformed id answers
  400, an unknown server 404, and an inactive one 404 for anyone but an administrator; otherwise it
  opens with the current snapshot (read through `status_broadcast::SELECT_SERVER_STATUS`, the same
  projection the publishers use), then relays every frame the realtime hub fans out on
  `server:{id}`.

## Boundaries

- Depends on: the domain's models and services; `api_caller_identity`
  (`authorize_on_connection`, `lock_accounts`, `MachineCaller`); `api_community_content`
  (`load_modpack`, `ModpackDto`, `Modpack`); `api_mission_vocabulary::TerrainType`;
  `api_audit_log` (audit writers); `api_http_layer` for the extractors, `role_rank` and the
  authorized [SSE](/documentation/glossary/n_to_z.md#sse) stream; `api_foundation` for errors.
- Used by: the domain's `routes.rs`; over HTTP, the server control page and its fleet command,
  deployment and credential panels in `crates/frontend/pages/administration_pages/src/server_control/`,
  the server intel page in `crates/frontend/pages/command_center_pages/src/server_intel/`, the
  host agent's ledger client in `crates/fleet/game_server_host_agent/src/ledger_client/`, and the game runtime's
  [API](/documentation/glossary/a_to_f.md#api) scripts in
  `mod/tbd-framework/Scripts/Game/TBD/API/`.
- Rules: every handler carries its `/// @route` tag; no handler
  imports another domain's handlers; the
  server writes live under `/api/v1/servers`, not `/api/v1/admin/servers`, because every
  signed-in member may read the servers.
- Body decoding: every JSON body is read through `ApiError::from_json_rejection`: 413 with
  `details.code = request_too_large` over the body limit, 415 without a JSON content type, and 400
  with the decoder's message (which names the failing field) otherwise. The game-runtime session
  start treats an empty body as no loaded artifact and reads any other body the same way.
- Path decoding: every path segment is read through `api_foundation::http::path_parameters::PathParams`: a
  segment that does not decode into its type answers 400 in the `{error}` envelope with a message
  naming the parameter, never axum's plain-text rejection.
