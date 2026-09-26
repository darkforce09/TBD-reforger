# Fleet host agent source

The `fleet_host_agent` library and the `fleet-host-agent` binary: everything the
[fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent) does between reading its
configuration and reporting a [fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command)'s
outcome to the [API](/documentation_v2/glossary/a_to_f.md#api).

## Contents

```text
apps/fleet_host_agent/src/
├── action_verdict.rs         `ActionVerdict`, an action's observed outcome in the ledger's shape
├── agent_configuration/      the TOML configuration, its validation and the two secret files
├── command_execution/        the second validation of each claimed command, and the host's executor
├── dedicated_server_config/  the surgical, atomic rewrite of `game.scenarioId` in the server config
├── ledger_client/            the claim loop and the HTTP calls to the API's fleet executor routes
├── lib.rs                    the module tree of the `fleet_host_agent` library
├── main.rs                   the binary: command line, logging, wiring and shutdown on SIGTERM or SIGINT
├── process_control/          systemctl invocations and the unit-state verdict
├── rcon/                     the BattlEye RCon client and the Reforger commands it sends
├── secret_text.rs            `SecretText`, a secret whose `Debug` output is redacted and that has no `Display`
└── tests/                    unit tests for the verdict and the secret text
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

- The `fleet-host-agent` binary (`main.rs`), described in the crate README.
- The library `fleet_host_agent` (`lib.rs`) makes every module public: `action_verdict`,
  `agent_configuration`, `command_execution`, `dedicated_server_config`, `ledger_client`,
  `process_control`, `rcon` and `secret_text`. Its users are the binary and the integration tests
  in `apps/fleet_host_agent/tests/`.

## Boundaries

- Depends on: the crates `apps/fleet_host_agent/Cargo.toml` declares; at run time the API's
  `/api/v1/fleet-executor/` routes, the user's systemd manager, the server's RCON port and its
  JSON config.
- Used by: the integration tests in `apps/fleet_host_agent/tests/`; no other crate depends on the
  library.
- Rules: nothing prints a secret (`tests/secret_text.rs`); a failure reason never exceeds 512
  bytes (`tests/action_verdict.rs`); `cargo xtask verify file-length` holds this folder and
  `apps/fleet_host_agent/tests/` to the file-size limits.
