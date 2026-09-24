# Server infrastructure handlers

The HTTP handlers of the game server fleet: the server intel reads and the registry writes,
machine credentials, the fleet command ledger for operators and for executors, runtime sessions,
the fleet scenario registry and the live status stream.

## Contents

```text
apps/website/api_v2/src/server_infrastructure/handlers/
├── fleet_commands.rs         administrator command requests, receipts and cancellation
├── fleet_executor.rs         executor claims, start reports and outcome reports, by machine credential
├── fleet_scenarios.rs        the registry of the mission header the fleet boots for each terrain
├── game_runtime_sessions.rs  a game runtime starts and ends its session (`mod_runtime` credential)
├── machine_credentials.rs    administrator issue, list and revocation of machine credentials
├── mod.rs                    declares one module per fleet surface
├── server_intel.rs           the server intel reads: each server with its status, modpack and terrain
├── server_registry.rs        create, partially update and deactivate a `servers` row
├── server_status_stream.rs   the SSE feed of one server's live status
└── tests/                    unit tests for the scenario validation and the intel card
```

## How it works

Three tiers meet here. Members read the server intel and the status stream with `AuthUser`.
Administrators (`AdminUser`) write the registry, the credentials, the commands and the
[fleet scenarios](/documentation_v2/glossary.md#fleet-scenario); every write that changes a
credential, a command or a scenario takes its locks, rechecks the administrator on that
transaction (`authorize_on_connection`) and audits inside it. The programs on a game host present a
[machine credential](/documentation_v2/glossary.md#machine-credential) (`MachineCaller`): the
[fleet host agent](/documentation_v2/glossary.md#fleet-host-agent) and the game runtime claim and
report commands, and the game runtime starts and ends its runtime session, reporting the artifact
it loaded; each acts only for its own server and executor kind.

- `server_intel.rs` composes one card per server (the registration, its live status row, the
  modpack it requires and the terrain of the match it runs) for the list and the single read alike.
- `server_registry.rs` validates at the boundary what the `servers` table does not constrain: a
  trimmed, non-blank name, the address, the port and an existing modpack; an explicit `null` for
  `required_modpack_id` clears the modpack, and an absent key keeps it.
- `fleet_commands.rs` answers 202 with the receipt, and 400 for `load_mission` and
  `restart_with_mission`, which only a mission deployment issues; `fleet_executor.rs` answers 204
  when nothing is claimable.
- `fleet_scenarios.rs` keys a scenario by a terrain key (lowercase ASCII, digits and underscores,
  starting with a letter, at most 64 bytes) and a `{16 uppercase hex}` resource ending in `.conf`.
- `server_status_stream.rs` opens with the current snapshot, then relays every frame the realtime
  hub fans out on `server:{id}`.

## Boundaries

- Depends on: the domain's models and services; `identity_and_access` (`authorize_on_connection`,
  `lock_accounts`); `community_content` (`load_modpack`, `ModpackDto`, `Modpack`);
  `missions::models::mission::TerrainType`; `administration` (audit writers); `core` for the
  extractors, `role_rank`, errors and the authorized event stream.
- Used by: the domain's `routes.rs`; over HTTP, the server control page and its fleet command,
  deployment and credential panels in `apps/website/frontend/src/v2/pages/administration/server_control/`,
  the server intel page in `apps/website/frontend/src/v2/pages/command_center/server_intel/`, the
  host agent's ledger client in `apps/fleet_host_agent/src/ledger_client/`, and the game runtime's
  API scripts in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`apps/website/api_v2/src/tests/architecture_rules.rs`); the
  server writes live under `/servers`, not `/admin/servers`, because every signed-in member may
  read the servers.
