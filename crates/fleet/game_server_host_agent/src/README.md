# Game server host agent source

The `game_server_host_agent` library and the `game_server_host_agent` binary: everything the
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) does between reading its
configuration and reporting a [fleet command](/documentation/glossary/a_to_f.md#fleet-command)'s
outcome to the [API](/documentation/glossary/a_to_f.md#api).

## Contents

```text
crates/fleet/game_server_host_agent/src/
├── action_verdict.rs         `ActionVerdict`, an action's observed outcome in the ledger's shape
├── agent_configuration/      the TOML configuration, its validation and the two secret files
├── command_execution/        the second validation of each claimed command, and the host's executor
├── dedicated_server_config/  the surgical, atomic rewrite of `game.scenarioId` in the server config
├── error.rs                  `Error` and `Result`: the crate's one error, wrapping each module's own
├── identifiers.rs            `SessionPlayerId`, `ArmaPlayerId` and `ScenarioId`, the typed ids the agent reads and writes
├── ledger_client/            the claim loop and the HTTP calls to the API's fleet executor routes
├── lib.rs                    the module tree of the `game_server_host_agent` library
├── main.rs                   the binary: command line, logging, wiring and shutdown on SIGTERM or SIGINT
├── prelude.rs                the names a caller imports with `use game_server_host_agent::prelude::*;`
├── process_control/          systemctl invocations and the unit-state verdict
├── rcon/                     the BattlEye RCon client and the Reforger commands it sends
├── secret_text.rs            `SecretText`, a secret whose `Debug` output is redacted and that has no `Display`
└── tests/                    unit tests for the verdict, the secret text and the typed ids
```

## How it works

`main.rs` loads the configuration (`agent_configuration`), starts the RCON client (`rcon`),
builds the ledger client (`ledger_client::LedgerApi`) and the process control
(`process_control`), and hands a `command_execution::HostActionExecutor` to
`ledger_client::CommandLoop`, which runs until SIGTERM or SIGINT. Then it sends `@logout` over
RCON and exits 0.

```text
main.rs ── agent_configuration ──▶ settings and secrets
   │
   ▼
ledger_client::CommandLoop ── claim, executing, result ──▶ API /api/v1/fleet-executor/
   │ HostCommand::from_claim (command_execution)
   ▼
HostActionExecutor ──▶ process_control ──▶ systemctl --user
                   ──▶ rcon ──▶ the server's RCON port
                   ──▶ dedicated_server_config + process_control (restart_with_mission)
   │
   ▼
ActionVerdict ──▶ ExecutionResult (succeeded, outcome, failure_reason)
```

`ActionVerdict` holds a success with an outcome object, or a failure with a reason and, when
something was observed, an outcome as well; a failure reason is trimmed and cut to the ledger's
512-byte limit on a character boundary, and an empty one reads "the action failed". The machine
credential and the RCON password travel as `SecretText` and are exposed only where they enter
their own protocol.

## Public surface

- The `game_server_host_agent` binary (`main.rs`), described in the crate README.
- The library `game_server_host_agent` (`lib.rs`) makes every module public: `action_verdict`,
  `agent_configuration`, `command_execution`, `dedicated_server_config`, `error`, `identifiers`,
  `ledger_client`, `prelude`, `process_control`, `rcon` and `secret_text`, and re-exports `Error`
  and `Result` at its root. Its users are the binary and the integration tests in
  `crates/fleet/game_server_host_agent/tests/`.
- Errors: each module returns its own error (`ConfigurationError`, `CommandRefusal`,
  `ServerConfigError`, `SystemdUnitNameProblem`, `RconError`, `LedgerApiSetupError`,
  `LedgerError`), because its callers branch on the variants; `Error` wraps every one of them
  with the wrapped error's message, and `RconClient::start` returns `Result` with
  `Error::RconSocket` for a socket that cannot be opened.
- Typed ids: a `#players` row carries a `SessionPlayerId` and an `ArmaPlayerId`, which serialise
  as their bare number and text, so the `list_players` outcome keeps its JSON;
  `DedicatedServerConfig::switch_scenario` takes a validated `ScenarioId`.

## Boundaries

- Depends on: the crates `crates/fleet/game_server_host_agent/Cargo.toml` declares; at run time the API's
  `/api/v1/fleet-executor/` routes, the user's systemd manager, the server's RCON port and its
  JSON config.
- Used by: the integration tests in `crates/fleet/game_server_host_agent/tests/`; no other crate depends on the
  library.
- Rules: nothing prints a secret (`tests/secret_text.rs`); a failure reason never exceeds 512
  bytes; `cargo xtask verify file-length` holds this folder and
  `crates/fleet/game_server_host_agent/tests/` to the file-size limits.
