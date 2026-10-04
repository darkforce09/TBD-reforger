# Server infrastructure models

The rows and wire shapes of the game server fleet: the server registration and its live status, the
[fleet commands](/documentation/glossary/a_to_f.md#fleet-command) and their receipts, the
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential) without their secrets, and
the [fleet scenario registry](/documentation/glossary/a_to_f.md#fleet-scenario). Keys are snake_case,
absent values are skipped and timestamps are RFC 3339.

## Contents

```text
crates/api/api_server_infrastructure/src/models/
├── fleet_command.rs       `FleetCommandState` and `FleetCommandReceiptRow`, the stored command row of a receipt
├── fleet_scenario.rs      `FleetScenario`, one terrain's mission header, with its update body and list
├── machine_credential.rs  the credential views, issue body, revocation and list
├── mod.rs                 the module tree
└── server.rs              `Server`, its live `ServerStatus` with the telemetry queue, the history
```

## How it works

The fleet command wire shapes (`FleetAction` with its rules, the request, receipt, list, claim,
reports and console types) and `ExecutorKind` live in the `fleet_wire_contract` crate
(`crates/contracts/fleet_wire_contract`), which the fleet host agent shares; the ledger reads a
`FleetCommandReceiptRow` from the `fleet_commands` table and converts it into that crate's
`FleetCommandReceipt` column for column.
`FleetCommandState` runs from `queued` through `claimed` and `executing` to `succeeded`, `failed`,
`expired`, `cancelled` or `indeterminate`. `ServerStatus` is read through `ServerStatusRow`, the flat
`server_statuses` projection the `server_status_columns!` macro spells once for every query; the
conversion folds the five `telemetry_queue_*` columns, all set or all null, into the optional
`TelemetryQueueStatus` (`backlog`, `capacity`, `dropped_total`, `oldest_age_seconds`,
`reported_at`), which serialises only when the server has reported a queue. A `MachineCredential` never carries its secret; only
`IssuedMachineCredential`, the issue answer, does, once.

## Boundaries

- Depends on: `fleet_wire_contract::rfc3339_timestamps` for timestamps, `fleet_wire_contract` for the receipt and
  `ExecutorKind`, serde and sqlx; `contract_schema_types::server_infrastructure` holds the types
  generated from `contracts/definitions/fleet-command.schema.json`,
  `contracts/definitions/game-runtime-session.schema.json` (whose `RuntimeHeartbeat` carries the
  optional `telemetry_queue` block) and `contracts/definitions/machine-credential.schema.json`;
  `TelemetryQueueStatus` cites `match-telemetry.schema.json#/definitions/TelemetryQueueStatus`.
  `machine_credential.rs` and `fleet_scenario.rs` carry `@contract` tags for the machine
  credential and `mission-deployment.schema.json` fleet scenario definitions they serialize (the
  fleet command tags sit on the `fleet_wire_contract` types), and `server.rs` also cites
  `server-intel.schema.json` (`ServerStatus`, `TelemetryQueueStatus`); `cargo xtask schema
  citations` resolves every tag.
- Used by: the domain's handlers and services and `api_command_center` (`ServerStatus`,
  `ServerStatusRow`); the contract test `apps/api/tests/game_runtime_contract.rs`, which decodes
  live answers into the generated types; the web app's DTOs in
  `crates/frontend/foundation/frontend_api_dtos/src/` (`servers.rs`, `fleet_commands.rs`,
  `fleet_scenarios.rs`) mirror these shapes.
- Rules: the generated types are written by `cargo xtask ci schema-codegen` and never edited by
  hand (`cargo xtask ci verify-codegen-fresh` checks them); a new action gets its executor, idempotence,
  process and execution-window answers in `fleet_wire_contract`'s `fleet_action.rs`, its
  argument rules in the ledger's `command_arguments.rs` and, when its outcome has a contract, its outcome rules in
  `command_outcomes.rs`.
