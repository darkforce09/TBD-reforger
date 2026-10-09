# Server infrastructure services

The logic behind the game server fleet: the issue, list and revocation of the
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential) that authenticate the
programs on a game host (verifying a presented one, and the `MachineCaller` extractor, live in the
caller identity crate, `crates/api/api_caller_identity/src/`), the runtime sessions that fence each boot of a
[game runtime](/documentation/glossary/g_to_m.md#game-runtime), the
[fleet command](/documentation/glossary/a_to_f.md#fleet-command) ledger, and the publisher of each
server's live status topic.

## Contents

```text
crates/api/api_server_infrastructure/src/services/
├── fleet_commands/         the durable fleet command ledger: requests, claims, reconciliation
├── machine_credentials.rs  issue, list and revoke machine credentials
├── mod.rs                  the module tree
├── runtime_sessions.rs     generation-fenced runtime sessions: start, heartbeat, share, end, expiry
├── server_registration.rs  the server field rules and the insert with its `server.create` audit
├── status_broadcast.rs     the `server:{id}` SSE topic: the live-status query and its publishers
└── tests/                  unit tests for credential labels and the status payload
```

## How it works

A machine secret reads `tbdm_<credential id>_<64 hex digits>`. `machine_credentials.rs` issues
it once, storing only its SHA-256, for one server and one executor kind (`host_agent` or
`mod_runtime`), and revoking a credential ends every runtime session it authenticated. The
caller identity crate's `machine_caller.rs` verifies a presented secret into the `MachineCaller` every machine
handler checks the resources it touches against.

`server_registration.rs` is the one path that creates a `servers` row: `ServerRegistration::new`
trims the name and refuses a blank one, a hostname or masked address and a port outside 1 to 65535,
and `register_server` checks the required modpack, inserts the row and appends its `server.create`
audit row on the caller's transaction, so both commit together.

A runtime session is one boot of a server's game runtime. Starting one ends the open session as
superseded and takes the next generation; a heartbeat must name that generation and a sequence
that strictly increases, every 15 seconds, and a session silent for 60 seconds expires and marks its
server offline. Ending a session ends the player lives still open in it. Lock order: server, runtime
session rows, their live occupancies. `status_broadcast.rs` is the only writer of the `server:{id}`
topic on `api_http_layer::realtime_hub`, so the in-request publish, the scheduled republish and the stream's
first snapshot carry one payload shape: every read goes through the `ServerStatusRow` projection,
which folds the five telemetry queue columns into `ServerStatus.telemetry_queue` (absent when the
server never reported a queue). `publish_all_server_statuses` republishes the configured fleet
(active servers) only, through `SELECT_FLEET_STATUSES`, which the dashboard's fleet overview reads
too. The ledger in `fleet_commands/` has its own README.

## Public surface

- `server_registration`: `ServerRegistration` and `register_server` for `POST /servers` in this
  domain's handlers and for the `staging-fixtures provision-fleet` host tool; the field validators
  for `PATCH /servers/{id}`.
- `machine_credentials`: `issue_machine_credential` for the credential routes and for the
  `staging-fixtures` host tool, which writes each secret into a mode-600 file;
  `list_machine_credentials` and `revoke_machine_credential` for the credential routes.
- `runtime_sessions`: `admit_heartbeat` and `HeartbeatFence` for the heartbeat in `api_match_telemetry`;
  `share_open_session` for the live [slot](/documentation/glossary/n_to_z.md#slot) occupancy in
  `api_operations`; `expire_silent_runtime_sessions` for the `runtime_session_expiry` worker.
- `status_broadcast`: `publish_server_status_by_id` for the heartbeat in `api_match_telemetry` and the
  `runtime_session_expiry` worker, `publish_all_server_statuses` for the `server_status_publisher`
  worker, `publish_server_status` for a status already read; `SELECT_SERVER_STATUS` for the
  stream's snapshot and `SELECT_FLEET_STATUSES` for `api_command_center::services::fleet_overview`.
- `fleet_commands::command_ledger`: `enqueue_deployment_command` and `cancel_command` for
  [mission deployments](/documentation/glossary/g_to_m.md#mission-deployment) in `api_missions`;
  `fleet_commands::command_reconciliation::reconcile_fleet_commands` for the
  `fleet_command_reconciler` worker.

## Boundaries

- Depends on: the domain's models; `api_http_layer` (authentication primitives, the realtime
  hub); `api_foundation` (errors); `fleet_wire_contract` (wire formats);
  `api_audit_log::required_audit`; `api_caller_identity` (the account authority and
  `MachineCaller`).
- Used by: the domain's handlers; `api_match_telemetry`, `api_missions`, `api_operations` and
  `api_command_center` through the surface above; the workers in `crates/api/api_background_workers/src/`;
  the `staging-fixtures` host tool in `tools/staging/staging_fixtures/`; the integration tests in
  `crates/api/api_server/tests/`.
- Rules: only the SHA-256 of a secret is stored, and the secret is shown once at issue; a machine
  acts only for its own server; `status_broadcast.rs` stays the one publisher of the `server:{id}`
  topic.

## Related documentation

- [Machine credentials and runtime sessions](/documentation/crates/api/api_server/verification_evidence/machine_credentials.md)
  — the credential format, the session fence and their consumers.
- [Fleet command ledger](/documentation/crates/api/api_server/verification_evidence/fleet_command_ledger.md)
  — the ledger's design.
