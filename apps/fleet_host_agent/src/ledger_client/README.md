# Ledger client

The executor side of the [API](/documentation_v2/glossary/a_to_f.md#api)'s
[fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command) ledger: claim the next command for
this server, report that its effect is starting, perform it, and report its outcome, over
outbound HTTPS with this host's [machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential).

## Contents

```text
apps/fleet_host_agent/src/ledger_client/
├── command_loop.rs     `CommandLoop`, the claim loop and its reporting rules, and `LedgerTimings`
├── ledger_api.rs       `LedgerApi`, the three HTTP calls, and the `LedgerError` classification of failures
├── ledger_messages.rs  the wire messages of the executor routes and the API's error envelope
├── mod.rs              the module tree; re-exports the loop, the client, the messages and the backoff
├── retry_backoff.rs    `JitteredBackoff`, exponential backoff drawn from the upper half of each step
└── tests/              unit tests for the failure classification and the backoff
```

## How it works

`CommandLoop::run` repeats until the shutdown future completes, and checks it only between
commands, so a command in progress is performed and reported first:

```text
claim ──204──▶ wait poll_interval ──▶ claim
  │ 200: ClaimedFleetCommand (command_id, action, arguments, fencing_token, lease_expires_at)
  ▼
HostCommand::from_claim ──refused──▶ result {succeeded: false} ──▶ claim
  │
  ▼
executing {fencing_token} ──409 or other refusal──▶ abandon without acting ──▶ claim
  │ acknowledged (transient failures retried with backoff first)
  ▼
FleetActionExecutor::execute ──▶ result {succeeded, outcome, failure_reason} ──▶ claim
```

The three calls are `POST /api/v1/fleet-executor/commands/claim` with the body `{}`,
`POST /api/v1/fleet-executor/commands/{commandId}/executing` and
`POST /api/v1/fleet-executor/commands/{commandId}/result`, whose messages follow
`contracts_v2/definitions/fleet-command.schema.json`; an absent outcome or failure reason is left
out of the JSON rather than sent as null.

`classify_refusal` sorts every answer that is not a success: a 409 whose `details.code` is
`STALE_FENCING_TOKEN` means the claim was taken away; a transport failure, 408, 429 or 5xx is
transient (`LedgerError::is_transient`); anything else is a final refusal. The rules the loop
keeps:

- Nothing runs without an acknowledged `executing` report. A transient failure of that report is
  retried before acting; a stale fencing token or any other refusal abandons the command, which
  is never reported again.
- A command that fails the second validation is reported failed straight from its claimed state,
  before any effect.
- The result report is retried with backoff until the API acknowledges it or refuses it for good.
  The effect itself is never repeated: when the outcome of a command that is not idempotent stays
  unreported, the ledger marks it indeterminate and an operator decides.
- Commands run one at a time, and every step is logged inside a span carrying the command id,
  action and fencing token.

| Timing | Value |
|---|---|
| Wait after a 204 | `poll_interval_seconds` from the configuration |
| Backoff after a failed claim | 1 s doubling to 60 s |
| Backoff between report attempts | 500 ms doubling to 15 s |
| Connect and request timeouts | 10 s and 20 s |

Each backoff delay is drawn uniformly from the upper half of its step, so agents that failed
together do not retry together. `LedgerApi::new` sends the credential only as a sensitive
`Authorization: Bearer` header, never follows a redirect, requires HTTPS whenever the origin is
HTTPS, and installs the `ring` TLS provider once.

## Boundaries

- Depends on: `crate::command_execution` (`HostCommand`, `FleetActionExecutor`),
  `crate::action_verdict` and `crate::secret_text`; the `reqwest` (rustls without a bundled
  provider), `rustls`, `serde`, `serde_json`, `chrono`, `uuid`, `rand`, `tokio` and `tracing`
  crates; the API's `/api/v1/fleet-executor/` routes in
  `apps/website/api_v2/src/server_infrastructure/routes.rs`.
- Used by: `apps/fleet_host_agent/src/main.rs`, which runs the loop until SIGTERM or SIGINT, and
  `apps/fleet_host_agent/tests/host_agent_ledger.rs`, which runs it against a stand-in of the
  executor routes.
- Rules: the executing-before-effect, stale-claim, retry and refusal rules above
  (`tests/ledger_api.rs` and the `host_agent_ledger_*` tests); the messages match
  `contracts_v2/definitions/fleet-command.schema.json`.

## Related documentation

- [Fleet command ledger](/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md)
  — the API side of the ledger: states, leases, fencing and execution windows.
