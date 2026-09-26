# Server infrastructure models

The rows and wire shapes of the game server fleet: the server registration and its live status, the
[fleet commands](/documentation_v2/glossary/a_to_f.md#fleet-command) and their receipts, the
[machine credentials](/documentation_v2/glossary/g_to_m.md#machine-credential) without their secrets, and
the [fleet scenario registry](/documentation_v2/glossary/a_to_f.md#fleet-scenario). Keys are snake_case,
absent values are skipped and timestamps are RFC 3339.

## Contents

```text
apps/website/api_v2/src/server_infrastructure/models/
├── fleet_command.rs       `FleetAction`, `FleetCommandState` and the command ledger's wire shapes
├── fleet_scenario.rs      `FleetScenario`, one terrain's mission header, with its update body and list
├── generated/             types generated from the fleet command, session and credential schemas
├── machine_credential.rs  `ExecutorKind` and the credential views, issue body, revocation and list
├── mod.rs                 the module tree
└── server.rs              `Server`, its one live `ServerStatus` row and `ServerStatusHistory`
```

## How it works

`FleetAction` holds the rules of each action: which executor performs it (`broadcast`, `kick` and
`load_mission` run in the [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime), everything
else on the host agent, since Reforger's [RCON](/documentation_v2/glossary/n_to_z.md#rcon) has no
broadcast), whether repeating it is harmless (`start`, `stop`, `list_players`), whether it changes
the server process (at most one such command runs per server), whether only a
[mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment) may issue it (`load_mission`,
`restart_with_mission`), and how long its execution may take (30 to 180 seconds).
`FleetCommandState` runs from `queued` through `claimed` and `executing` to `succeeded`, `failed`,
`expired`, `cancelled` or `indeterminate`. A `MachineCredential` never carries its secret; only
`IssuedMachineCredential`, the issue answer, does, once.

## Boundaries

- Depends on: `core::wire_format` for timestamps, serde and sqlx; `generated/` follows
  `contracts_v2/definitions/fleet-command.schema.json`,
  `contracts_v2/definitions/game-runtime-session.schema.json` and
  `contracts_v2/definitions/machine-credential.schema.json`.
- Used by: the domain's handlers and services; `match_telemetry` (`ExecutorKind`, `ServerStatus`),
  `missions` (`ExecutorKind`, `FleetAction`), `operations` (`ExecutorKind`) and `command_center`
  (`ServerStatus`); the contract test `apps/website/api_v2/tests/game_runtime_contract.rs`, which
  decodes live answers into the generated types; the web app's DTOs in
  `apps/website/frontend/src/v2/core/api/dto/` (`servers.rs`, `fleet_commands.rs`,
  `fleet_scenarios.rs`) mirror these shapes.
- Rules: `generated/` is written by `cargo xtask ci schema-codegen` and never edited by hand
  (`cargo xtask ci verify-codegen-fresh` checks it); a new action gets its executor, idempotence,
  process and execution-window answers here and its argument rules in the ledger's
  `command_arguments.rs`.
