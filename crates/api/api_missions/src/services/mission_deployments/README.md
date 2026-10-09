# Mission deployments

The path from an approved [artifact](/documentation/glossary/a_to_f.md#artifact) to a running game
server. A [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) is validated before
anything is stored, performed by one [fleet command](/documentation/glossary/a_to_f.md#fleet-command),
and confirmed only when a runtime session of that server reports the artifact it loaded.

## Contents

```text
crates/api/api_missions/src/services/mission_deployments/
├── deployment_reads.rs       a server's deployments as operators see them, and the one a runtime runs
├── deployment_requests.rs    request and cancel a deployment, with its command, bindings and audit
├── deployment_selection.rs   validate a selection before anything is stored, and pick its transition
├── deployment_settlement.rs  confirm or fail the deployment in flight, for one server or all of them
├── mod.rs                    the module tree
├── slot_bindings.rs          pair an event mission's ORBAT seats with the artifact's compiled slots
└── tests/                    unit tests for the seat-to-slot binding
```

## How it works

`request_deployment` serves the administrator's route and the
[game runtime](/documentation/glossary/g_to_m.md#game-runtime)'s relayed in-game request alike, in one
transaction: it locks the server and settles its
[deployment](/documentation/glossary/a_to_f.md#deployment) in flight, validates the selection
(`validate_selection` locks the [mission](/documentation/glossary/g_to_m.md#mission), then the
[event](/documentation/glossary/a_to_f.md#event) mission when one is named), locks the requester's
account and rechecks their administrator authority, then records the fleet command, the
`mission_deployments` row, its [slot](/documentation/glossary/n_to_z.md#slot) bindings and the audit
row. In a test build it passes the failpoint `DeploymentRequestBeforeCommit` after the audit row;
both requesting handlers pass `DeploymentRequestAfterCommit` after their commit.

A refused selection stores nothing and answers with a `code`:

| Code | Status | Cause |
|---|---|---|
| `SERVER_INACTIVE` | 409 | the server is deactivated |
| `DEPLOYMENT_IN_PROGRESS` | 409 | another deployment of the server is in flight |
| `ARTIFACT_NOT_APPROVED` | 409 | the mission is not live, or its latest approval decided another artifact |
| `MODPACK_MISMATCH` | 422 | the artifact was compiled against another modpack than the server requires |
| `TERRAIN_NOT_RUNNABLE` | 422 | no [fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario) is registered for the artifact's terrain |
| `EVENT_MISSION_NOT_ON_SERVER` | 409 | the event mission runs another mission, or its event names another server |
| `ORBAT_ARTIFACT_MISMATCH` | 422 | the event mission's seats and the artifact's slots do not pair one to one |
| `IDENTITY_NOT_LINKED` | 403 | the in-game requester's Arma identity is linked to no account |
| `NOT_AN_ADMINISTRATOR` | 403 | the requester holds no administrator authority |

A deployment to a server whose open runtime session reported an artifact of the same terrain is a
`scenario_restart` (a `load_mission` command for that session's game runtime, 600 s to confirm);
any other is a `host_restart` (a `restart_with_mission` command for the host agent, carrying the
terrain's fleet scenario, 1200 s to confirm).

`slot_bindings.rs` pairs each [ORBAT](/documentation/glossary/n_to_z.md#orbat) slot of the event
mission with the compiled slot at the same faction, squad and position, read from the artifact's
own version payload, when both carry the same role; a role authored empty counts as `unassigned`,
as the compiler writes it. The bindings are stored in `mission_deployment_slots`, where the event
roster and deployment authorization in `api_operations` read them, so game loading, roster derivation
and seat authorization name one artifact.

`settle_server_deployment` decides the deployment in flight. The first runtime session started
after the request that reports a loaded artifact confirms it with the deployment's artifact id and
document SHA-256, and fails it with anything else. Without such a report, a command that ended
`failed`, `expired`, `cancelled` or `indeterminate` fails it, and so does a passed deadline, as a
partial transition. Settlement runs under the server row lock, before each request and cancel,
before every deployment read, and in `reconcile_mission_deployments`; every outcome is audited.

## Boundaries

- Depends on: `models::mission_deployment`; `api_audit_log` for the audit rows;
  `api_caller_identity` for account locks and administrator authority;
  `api_server_infrastructure` for the command ledger (`enqueue_deployment_command`, `cancel_command`);
  `fleet_wire_contract` for `FleetAction`; `mission_model::orbat` for the ORBAT template; `api_configuration`
  for configuration, `api_foundation` for errors and `api_http_layer` for `role_rank`.
- Used by: the handlers `mission_deployments.rs` and `game_runtime_missions.rs` in
  `crates/api/api_missions/src/handlers/`; the event roster in
  `crates/api/api_operations/src/handlers/game_runtime_roster.rs` (`deployment_in_effect`,
  `lock_and_settle`); the `mission_deployment_reconciler` worker in
  `crates/api/api_background_workers/src/`, every 5 s; the integration test
  `crates/api/api_server/tests/mission_deployment_transitions.rs`.
- Rules: at most one deployment per server is in flight, checked under the server lock and held by
  the `mission_deployments_one_in_flight_per_server` index of
  `crates/api/api_database/migrations/0053_mission_deployments.sql`; a selection is validated before
  anything is stored
  (`mission_transitions_selection_is_validated_before_anything_is_persisted`); only a later session
  reporting the exact artifact confirms
  (`deployment_confirmation_requires_the_exact_artifact_from_a_later_session`), both in
  `crates/api/api_server/tests/mission_deployment_transitions.rs`; a compiled slot binds to at most
  one seat (`two_seats_at_one_position_bind_once` in `tests/slot_bindings.rs`).

## Related documentation

- [Mission artifacts, reviews and deployment](/documentation/crates/api/api_server/verification_evidence/mission_artifacts.md)
  — the deployment design, its refusals and the game runtime's side.
- [Live slot occupancy](/documentation/crates/api/api_server/verification_evidence/live_occupancy.md)
  — how deployment authorization reads the slot bindings.
