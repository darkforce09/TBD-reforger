# Server infrastructure models

The rows and wire shapes of the game server fleet: the server registration and its live status, the
[fleet commands](/documentation/glossary/a_to_f.md#fleet-command) and their receipts, the
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential) without their secrets, and
the [fleet scenario registry](/documentation/glossary/a_to_f.md#fleet-scenario). Keys are snake_case,
absent values are skipped and timestamps are RFC 3339.

## Contents

```text
apps/api/src/server_infrastructure/models/
├── fleet_command.rs       `FleetAction`, `FleetCommandState`, the ledger's wire shapes and console types
├── fleet_scenario.rs      `FleetScenario`, one terrain's mission header, with its update body and list
├── generated/             types generated from the fleet command, session and credential schemas
├── machine_credential.rs  `ExecutorKind` and the credential views, issue body, revocation and list
├── mod.rs                 the module tree
└── server.rs              `Server`, its live `ServerStatus` with the telemetry queue, the history
```

## How it works

`FleetAction` holds the rules of each action: which executor performs it (`broadcast`, `kick` and
`load_mission` run in the [game runtime](/documentation/glossary/g_to_m.md#game-runtime), everything
else on the host agent, since Reforger's [RCON](/documentation/glossary/n_to_z.md#rcon) has no
broadcast), whether repeating it is harmless (`start`, `stop`, `list_players`), whether it changes
the server process (at most one such command runs per server; a `console_command` counts, since a
console line can stop or restart the server), whether only a
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) may issue it (`load_mission`,
`restart_with_mission`), and how long its execution may take (30 to 180 seconds).
`ConsoleCommandArguments` is the stored console line (at most `LINE_MAX_BYTES`, 256) and
`ConsoleCommandOutcome` the reply a succeeded console command reports (at most
`RESPONSE_MAX_BYTES`, 4096, with `response_truncated`).
`FleetCommandState` runs from `queued` through `claimed` and `executing` to `succeeded`, `failed`,
`expired`, `cancelled` or `indeterminate`. `ServerStatus` is read through `ServerStatusRow`, the flat
`server_statuses` projection the `server_status_columns!` macro spells once for every query; the
conversion folds the five `telemetry_queue_*` columns, all set or all null, into the optional
`TelemetryQueueStatus` (`backlog`, `capacity`, `dropped_total`, `oldest_age_seconds`,
`reported_at`), which serialises only when the server has reported a queue. A `MachineCredential` never carries its secret; only
`IssuedMachineCredential`, the issue answer, does, once.

## Boundaries

- Depends on: `core::wire_format` for timestamps, serde and sqlx; `generated/` follows
  `contracts/definitions/fleet-command.schema.json`,
  `contracts/definitions/game-runtime-session.schema.json` (whose `RuntimeHeartbeat` carries the
  optional `telemetry_queue` block) and `contracts/definitions/machine-credential.schema.json`;
  `TelemetryQueueStatus` cites `match-telemetry.schema.json#/definitions/TelemetryQueueStatus`.
  `fleet_command.rs`, `machine_credential.rs` and `fleet_scenario.rs` carry `@contract` tags for
  the fleet command, machine credential and `mission-deployment.schema.json` fleet scenario
  definitions they serialize, and `server.rs` also cites `server-intel.schema.json`
  (`ServerStatus`, `TelemetryQueueStatus`); `cargo xtask schema citations` resolves every tag.
- Used by: the domain's handlers and services; `match_telemetry` (`ExecutorKind`),
  `identity_and_access` (`ExecutorKind`), `missions` (`ExecutorKind`, `FleetAction`), `operations`
  (`ExecutorKind`) and `command_center` (`ServerStatus`, `ServerStatusRow`); the contract test `apps/api/tests/game_runtime_contract.rs`, which
  decodes live answers into the generated types; the web app's DTOs in
  `apps/frontend/src/v2/core/api/dto/` (`servers.rs`, `fleet_commands.rs`,
  `fleet_scenarios.rs`) mirror these shapes.
- Rules: `generated/` is written by `cargo xtask ci schema-codegen` and never edited by hand
  (`cargo xtask ci verify-codegen-fresh` checks it); a new action gets its executor, idempotence,
  process and execution-window answers here, its argument rules in the ledger's
  `command_arguments.rs` and, when its outcome has a contract, its outcome rules in
  `command_outcomes.rs`.
