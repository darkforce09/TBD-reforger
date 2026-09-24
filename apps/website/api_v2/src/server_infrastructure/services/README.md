# Server infrastructure services

The logic behind the game server fleet: the
[machine credentials](/documentation_v2/glossary.md#machine-credential) that authenticate the
programs on a game host, the runtime sessions that fence each boot of a
[game runtime](/documentation_v2/glossary.md#game-runtime), the
[fleet command](/documentation_v2/glossary.md#fleet-command) ledger, and the publisher of each
server's live status topic.

## Contents

```text
apps/website/api_v2/src/server_infrastructure/services/
├── fleet_commands/            the durable fleet command ledger: requests, claims, reconciliation
├── machine_authentication.rs  the `MachineCaller` extractor every machine route takes
├── machine_credentials.rs     issue, list, revoke and verify machine credentials
├── mod.rs                     the module tree
├── runtime_sessions.rs        generation-fenced runtime sessions: start, heartbeat, share, end, expiry
├── status_broadcast.rs        the `server:{id}` SSE topic: the live-status query and its publishers
└── tests/                     unit tests for secret parsing, caller checks and the status payload
```

## How it works

A machine presents `Authorization: Bearer tbdm_<credential id>_<64 hex digits>`.
`machine_credentials.rs` selects the row by the id and compares the SHA-256 of the whole secret in
constant time, so a database read never yields a usable credential, and malformed, unknown and
mismatched secrets answer the same 401. The resulting `MachineCaller` names the one server and the
executor kind (`host_agent` or `mod_runtime`) the credential serves, and every handler checks the
resources it touches against that server. Revoking a credential ends every runtime session it
authenticated.

A runtime session is one boot of a server's game runtime. Starting one ends the open session as
superseded and takes the next generation; a heartbeat must name that generation and a sequence
that strictly increases, every 15 seconds, and a session silent for 60 seconds expires and marks its
server offline. Ending a session ends the player lives still open in it. Lock order: server, runtime
session rows, their live occupancies. `status_broadcast.rs` is the only writer of the `server:{id}`
topic on `core::realtime_hub`, so the in-request publish, the scheduled republish and the stream's
first snapshot carry one payload shape. The ledger in `fleet_commands/` has its own README.

## Public surface

- `machine_credentials::MachineCaller` with the extractor in `machine_authentication.rs`: taken by
  the machine routes of this domain, of `match_telemetry`, of `missions` and of `operations`.
- `runtime_sessions`: `admit_heartbeat` and `HeartbeatFence` for the heartbeat in `match_telemetry`;
  `share_open_session` for the live [slot](/documentation_v2/glossary.md#slot) occupancy in
  `operations`; `expire_silent_runtime_sessions` for the `runtime_session_expiry` worker.
- `status_broadcast`: `publish_server_status` for the heartbeat, `publish_server_status_by_id` for
  the `runtime_session_expiry` worker, `publish_all_server_statuses` for the
  `server_status_publisher` worker.
- `fleet_commands::command_ledger`: `enqueue_deployment_command` and `cancel_command` for
  [mission deployments](/documentation_v2/glossary.md#mission-deployment) in `missions`;
  `fleet_commands::command_reconciliation::reconcile_fleet_commands` for the
  `fleet_command_reconciler` worker.

## Boundaries

- Depends on: the domain's models; `core` (authentication primitives, errors, the realtime hub,
  wire formats); `administration::services::required_audit`;
  `identity_and_access::services::account_authority`.
- Used by: the domain's handlers; `match_telemetry`, `missions` and `operations` through the
  surface above; the workers in `apps/website/api_v2/src/background_workers/`; the integration
  tests in `apps/website/api_v2/tests/`.
- Rules: only the SHA-256 of a secret is stored, and the secret is shown once at issue; a machine
  acts only for its own server; `status_broadcast.rs` stays the one publisher of the `server:{id}`
  topic.

## Related documentation

- [Machine credentials and runtime sessions](/documentation_v2/website/api_v2/verification_evidence/machine_credentials.md)
  — the credential format, the session fence and their consumers.
- [Fleet command ledger](/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md)
  — the ledger's design.
