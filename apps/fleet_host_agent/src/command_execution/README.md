# Command execution

The [fleet commands](/documentation_v2/glossary/a_to_f.md#fleet-command) as this host performs them: the
second validation of every claimed command's action and arguments, and the executor that carries
out a validated command through process control, [RCON](/documentation_v2/glossary/n_to_z.md#rcon) or the
dedicated server's config.

## Contents

```text
apps/fleet_host_agent/src/command_execution/
├── command_refusal.rs       `CommandRefusal`, why a claimed command is refused without acting
├── host_action_executor.rs  `FleetActionExecutor` and the game host's `HostActionExecutor`
├── host_command.rs          `HostCommand::from_claim`, the action and argument check of a claim
├── mission_deployment.rs    `MissionDeployment` and `ScenarioId`, the `restart_with_mission` arguments
├── mission_restart.rs       `restart_with_mission`: switch the config's `scenarioId`, then restart
├── mod.rs                   the module tree; re-exports the refusal, command, deployment and executors
└── tests/                   unit tests for the command and mission deployment checks
```

## How it works

The [API](/documentation_v2/glossary/a_to_f.md#api) validated a command when it accepted it; the agent
checks it again, by the same rules, before anything reaches systemctl, the server config or RCON,
so no text from the API can widen what the host runs. `HostCommand::from_claim` accepts five
actions, each with exactly its own argument keys:

| Action | Arguments | Performed as | Succeeded when | Outcome keys |
|---|---|---|---|---|
| `start` | none | `ProcessAction::Start` | the unit is loaded and `active` after the dwell | `unit`, `load_state`, `active_state`, `dwell_milliseconds`, `systemctl` |
| `stop` | none | `ProcessAction::Stop` | the unit is loaded and `inactive` | as `start` |
| `restart` | none | `ProcessAction::Restart` | the unit is loaded and `active` after the dwell | as `start` |
| `list_players` | none | RCON `#players` | the server answered | `players` (`player_id`, `arma_id`, `name`), and `raw_lines` when a line was not understood |
| `restart_with_mission` | `MissionDeployment` | `mission_restart::restart_with_mission` | the config names the new mission header, and the restart succeeded | `scenario_id`, `config_path`, `unit_active_state` |

`broadcast`, `kick` and `load_mission` run in the
[game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime) and are refused as
`GameRuntimeAction`; any other action is `UnsupportedAction`. A refusal quotes at most 64
characters of the name it rejects.

A `restart_with_mission` command is a cross-terrain
[mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment): the server restarts on the
[mission header](/documentation_v2/glossary/g_to_m.md#mission-header) of another terrain. Its arguments
are exactly `deployment_id` and `artifact_id` (UUIDs in the 36-character hyphenated form),
`artifact_sha256` (64 lowercase hex digits) and `scenario_id`, a mission header resource matching
`^\{[0-9A-F]{16}\}[A-Za-z0-9_./-]+\.conf$` such as `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf`.
`restart_with_mission` then:

1. switches `game.scenarioId` in the server config on a blocking thread
   (`DedicatedServerConfig::switch_scenario`); a switch that reports an error fails the command
   with the file unchanged, nothing restarted and `config_updated: false`, and a switch whose
   thread did not finish fails it with nothing restarted;
2. restarts the unit exactly as `restart` does, with the same dwell and read-back verdict;
3. on a failed restart, fails the command with `config_updated: true`, `unit_load_state` and
   `systemctl` added, and a reason saying the config already names the new mission header, which
   the next start of the unit runs.

The agent never fetches the [artifact](/documentation_v2/glossary/a_to_f.md#artifact); the ids and the
digest only identify the deployment in the log. The game runtime reads its deployment when it
boots, loads and verifies the artifact and reports it, and that report confirms the deployment.

## Boundaries

- Depends on: `crate::process_control` (`ProcessControl`, `ProcessAction`), `crate::rcon`
  (`RconClient`, `reforger_commands::PLAYERS_COMMAND` and `parse_player_listing`),
  `crate::dedicated_server_config` and `crate::action_verdict`; the `serde_json`, `uuid`,
  `tokio` and `tracing` crates.
- Used by: `crate::ledger_client::command_loop`, which validates each claim with
  `HostCommand::from_claim` and runs it through a `FleetActionExecutor`;
  `apps/fleet_host_agent/src/main.rs`, which builds the `HostActionExecutor`; and the integration
  tests `apps/fleet_host_agent/tests/host_agent_ledger.rs` and
  `apps/fleet_host_agent/tests/process_control.rs`.
- Rules: the command loop calls `FleetActionExecutor::execute` only after the ledger acknowledged
  the `executing` report; the accepted actions and argument keys match the API's rules in
  `contracts_v2/definitions/fleet-command.schema.json` (`tests/host_command.rs` and
  `tests/mission_deployment.rs` hold every refusal); the `scenario_id` reaches the host only as a
  JSON string value in the config, never on a command line.
