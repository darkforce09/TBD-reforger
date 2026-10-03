# Fleet wire contract

The `fleet_wire_contract` crate: the shapes of `contracts/definitions/fleet-command.schema.json`
that the platform API and the fleet host agent exchange, each one type serving both serde
directions, plus `ExecutorKind`, the RFC 3339 spelling of their instants, the machine credential
format and the limits of the files a credential or RCON password is handed over in. The API
writes what the agent reads through the same type, so the two cannot drift apart.

## Contents

```text
crates/contracts/fleet_wire_contract/
├── Cargo.toml  the package: `chrono`, `serde`, `serde_json`, `thiserror`, `uuid`; layout tier 0
└── src/        the shapes, the instant spelling, the credential format, the file limits, the prelude
```

## How it works

An operator posts a `FleetCommandRequest`; the API answers every operator and executor route
with a `FleetCommandReceipt` (or a `FleetCommandList`), which it builds from its own stored row.
An executor claims a `ClaimedFleetCommand`, reports `ExecutionStart` before acting and
`ExecutionResult` after. `FleetAction` names each action and carries its rules (executor,
idempotence, process change, deployment-only, execution window); `ExecutorKind` names the program
that runs it. A console command stores `ConsoleCommandArguments` and reports
`ConsoleCommandOutcome`. Every instant goes through `rfc3339_timestamps`: UTC, a `Z` suffix and
fractional seconds trimmed of trailing zeros; an absent optional value is an absent key, and
reading accepts an absent key or null. The machine credential reads
`tbdm_<32 lowercase hex digits>_<64 lowercase hex digits>` (`check_machine_credential_format`),
and a secret file holds at most 4096 bytes with mode 600 and no group or other permission bits
(`secret_file_limits`).

## Getting started

Run from the repository root:

```bash
cargo test -p fleet_wire_contract   # golden JSON per shape and direction, the format check, the limits
```

## Configuration

None: the crate reads no environment variable and declares no feature.

## Public surface

- `fleet_action`: `FleetAction` (`ALL`, `as_str`, `parse`, `executor`, `idempotent`,
  `process_changing`, `deployment_only`, `execution_window_seconds`).
- `executor_kind`: `ExecutorKind` (`as_str`, `parse`).
- `operator_messages`: `FleetCommandRequest`, `FleetCommandReceipt`, `FleetCommandList`.
- `executor_messages`: `ClaimedFleetCommand`, `ExecutionStart`, `ExecutionResult`.
- `console_command`: `ConsoleCommandArguments` (`LINE_MAX_BYTES`), `ConsoleCommandOutcome`
  (`RESPONSE_MAX_BYTES`).
- `rfc3339_timestamps`: the `rfc3339_utc` and `rfc3339_utc_opt` serde-with modules and
  `rfc3339_utc::format`.
- `machine_credential_format`: `MACHINE_CREDENTIAL_PREFIX`, `CREDENTIAL_ID_HEX_DIGITS`,
  `CREDENTIAL_RANDOM_HEX_DIGITS`, `check_machine_credential_format`.
- `secret_file_limits`: `SECRET_FILE_MAX_BYTES`, `SECRET_FILE_MODE`, `SHARED_PERMISSION_BITS`.
- `Error` and `Result`: why a credential is malformed.
- `prelude`: the names most callers import.

## Boundaries

- Depends on: `chrono`, `serde`, `serde_json`, `thiserror` and `uuid`; no workspace crate.
- Used by: `apps/api` (the fleet command ledger and handlers, the machine credentials, every
  handler that requires an executor kind, and the staging fixtures tool's credential files),
  `apps/fleet_host_agent` (the ledger client and the secret-file reader) and `tools/xtask` (the
  credential prefix of the mod mission test).
- Rules:
  - no sqlx, axum or reqwest: the API keeps its database row (`FleetCommandReceiptRow`) and both
    sides keep their own file I/O; contracts tier, so the crate depends on no workspace crate
    outside the foundation tier (`cargo xtask verify crate-tiers`);
  - one type per shape: a change to a shape's serde attributes changes both directions at once,
    and the golden tests pin the exact bytes of each
    (`a_finished_receipt_carries_every_field_in_order`,
    `a_report_reads_null_as_absent_and_refuses_unknown_keys`);
  - the reports, the request and the console types refuse unknown keys; the claim and the receipt
    tolerate them;
  - every `@contract` tag resolves (`cargo xtask schema citations`).
- The single-page app keeps its own fleet command DTOs in `apps/frontend/src/v2/core/api/dto/`
  (decision D7 of the restructure program), checked against the same schema by its golden tests.

## Related documentation

- [Fleet command ledger](/documentation/apps/api/verification_evidence/fleet_command_ledger.md) —
  the states, the claim and fencing rules, and the reports these shapes carry.
- [Fleet command execution](/documentation/apps/fleet_host_agent/fleet_command_execution.md) —
  how the host agent claims, performs and reports a command.
