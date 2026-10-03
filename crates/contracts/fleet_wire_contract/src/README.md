# Fleet wire contract source

The source of `fleet_wire_contract`: one module per group of fleet shapes, the instant spelling
they share, the machine credential format and the secret-file limits, and the crate root that
declares them.

## Contents

```text
crates/contracts/fleet_wire_contract/src/
├── console_command.rs            `ConsoleCommandArguments` and `ConsoleCommandOutcome` with their size limits
├── error.rs                      `Error` and `Result`: why a machine credential is malformed
├── executor_kind.rs              `ExecutorKind`: host agent or game runtime, in its wire spelling
├── executor_messages.rs          `ClaimedFleetCommand`, `ExecutionStart` and `ExecutionResult`
├── fleet_action.rs               `FleetAction` and the rules each action carries
├── lib.rs                        the crate root: module header, `mod` lines and re-exports
├── machine_credential_format.rs  the `tbdm_` prefix, the hex part lengths and the format check
├── operator_messages.rs          `FleetCommandRequest`, `FleetCommandReceipt` and `FleetCommandList`
├── prelude.rs                    the names most callers import
├── rfc3339_timestamps.rs         the `rfc3339_utc` and `rfc3339_utc_opt` serde-with modules
├── secret_file_limits.rs         the size and permission limits of a secret file
└── tests/                        unit tests, one file per module
```

## How it works

`operator_messages` and `executor_messages` derive both `Serialize` and `Deserialize` on every
shape and apply `rfc3339_timestamps` to every instant, so the API's writes and the host agent's
reads (and the reverse for the reports) go through one definition. Optional fields combine
`default` with `skip_serializing_if`, so an absent value is written as an absent key and read
from an absent key or null. `fleet_action::FleetAction::executor` returns an
`executor_kind::ExecutorKind`. `machine_credential_format::check_machine_credential_format`
returns `error::Error::MalformedMachineCredential` for any text other than the credential format,
without the text itself.

## Boundaries

- Depends on: `chrono`, `serde`, `serde_json`, `thiserror` and `uuid`.
- Used by: the API's fleet ledger, handlers and machine credentials; the staging fixtures tool; the
  host agent's ledger client and secret-file reader; `xtask`'s mod mission test.
- Rules: no module imports sqlx, axum or reqwest; each shape module carries the `@contract` tags
  of the schema definitions it implements; tests live in `tests/`, one file per module.
