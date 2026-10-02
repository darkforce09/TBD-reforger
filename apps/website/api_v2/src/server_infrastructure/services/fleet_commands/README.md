# Fleet command ledger

The durable ledger of [fleet commands](/documentation/glossary/a_to_f.md#fleet-command): an operator's
command to one server is recorded before any executor can act on it, claimed under a lease and a
fencing token, reported as starting and finished, and reconciled by time when an executor goes
silent.

## Contents

```text
apps/website/api_v2/src/server_infrastructure/services/fleet_commands/
├── command_arguments.rs       per-action argument validation: each action accepts exactly its own keys
├── command_ledger.rs          the operator side: accept, cancel an unclaimed command, read receipts
├── command_outcomes.rs        per-action outcome validation: a console command's reply shape and bound
├── command_reconciliation.rs  expiry, lapsed leases and indeterminate outcomes, decided by time
├── executor_claims.rs         the executor side: claim under a lease, report the start and the outcome
├── mod.rs                     the module tree
└── tests/                     unit tests for the argument and outcome validation
```

## How it works

```text
queued ──claim──> claimed ──executing──> executing ──result──> succeeded | failed
  │                  │                       │
  │ 300 s unclaimed  │ 30 s lease lapses     │ execution window lapses
  v                  v                       v
expired           queued again,          idempotent: queued again
cancelled         new fencing token      otherwise: indeterminate
(operator, while queued)
```

`command_ledger.rs` accepts an operator's command with arguments that passed
`command_arguments.rs`; the actions only a [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment)
issues (`load_mission`, `restart_with_mission`) arrive through `enqueue_deployment_command`, which
commits with the deployment. `executor_claims.rs` hands the oldest claimable command to the
executor whose kind the action needs, checks that the requester still holds administrator
authority, and requires the current fencing token on every later report, so an executor whose
lease lapsed cannot overwrite a newer claim; a
[game runtime](/documentation/glossary/g_to_m.md#game-runtime) names its open runtime session, and a
command bound to another session fails. In a test build `record_result` passes the failpoint
`FleetCommandResultBeforeCommit` after the outcome's audit row; the executor handlers pass
`FleetCommandClaimAfterCommit` and `FleetCommandResultAfterCommit` after their commits.
`record_result` passes the reported outcome through `command_outcomes.rs` before writing it: a
succeeded `console_command` reports exactly `{response, response_truncated}` with a response of at
most 4096 bytes (a failed one reports that or nothing), any other console outcome answers 400 and
leaves the command as it was, and every other action's outcome is stored as reported.
`command_reconciliation.rs` runs every pass over all
servers, skipping rows another transaction holds. Lock order: server, runtime session, command rows.

## Boundaries

- Depends on: `models::fleet_command` and `models::machine_credential` of the domain,
  `services::machine_credentials::MachineCaller` and `services::runtime_sessions::share_open_session`;
  `identity_and_access::services::account_authority::holds_administrator_authority`;
  `administration::services::required_audit`; `core` for errors.
- Used by: the domain's `fleet_commands.rs` and `fleet_executor.rs` handlers; the mission
  deployment requests in `apps/website/api_v2/src/missions/services/mission_deployments/`
  (`enqueue_deployment_command`, `cancel_command`); the `fleet_command_reconciler` worker in
  `apps/website/api_v2/src/background_workers/`; the integration test
  `apps/website/api_v2/tests/fleet_command_ledger.rs`.
- Rules: no argument reaches a shell, and the one free-text argument, a console command's line,
  reaches only the server's RCON console; an executor receives only arguments that passed
  `command_arguments.rs`, and the ledger stores only outcomes that passed `command_outcomes.rs`;
  nothing repeats a non-idempotent command whose outcome is unknown, which becomes `indeterminate`
  for an operator to decide.

## Related documentation

- [Fleet command ledger](/documentation/website/api_v2/verification_evidence/fleet_command_ledger.md)
  — the ledger's states, rules and executors.
