# API background workers

The `api_background_workers` crate: the interval tasks the [API](/documentation/glossary/a_to_f.md#api)
binary arms at boot and never awaits. Each keeps shared state current when no request would:
expired credentials, the stored [event](/documentation/glossary/a_to_f.md#event) status, the
leaderboard view, live server status, silent
[game runtimes](/documentation/glossary/g_to_m.md#game-runtime),
[fleet commands](/documentation/glossary/a_to_f.md#fleet-command) and
[mission deployments](/documentation/glossary/g_to_m.md#mission-deployment) in flight, queued
reservation re-evaluations, unpublished audit facts, Discord membership and roles, and new
equipment export publications.

## Contents

```text
crates/api/api_background_workers/
├── Cargo.toml  the package: `api_state`, `api_http_layer`, `api_member_activity`, `api_equipment_datasets`, the five domain crates whose services the passes call, tokio, layout tier 10
└── src/        the thirteen worker modules, `spawn_all` and `WorkerHandles`, the error and the prelude
```

## How it works

`spawn_all(&state)` starts one Tokio task per worker against the application state and returns
`WorkerHandles`, one handle per worker. A worker owns its schedule, never its work: each pass calls
a service of the domain that owns the data, so a handler or a test reaches the same operation
without a timer. A failed pass is logged and the next one retries; the workers that poll stop once
the pool is closed. The source tree README has the table of workers, their intervals and the
services they call.

## Getting started

Run from the repository root:

```bash
cargo test -p api_background_workers
cargo clippy -p api_background_workers --all-targets -- -D warnings
cargo xtask db test-it --test eligibility_release_transactions --test runtime_session_fencing --test durable_rate_limit
```

The unit tests cover the three tunable intervals and the schedules of their workers; the passes
are proved against Postgres by the API's integration suites.

## Configuration

Three variables, read when `spawn_all` runs: `LEADERBOARD_REFRESH_INTERVAL_SECS`,
`SERVER_STATUS_PUBLISH_INTERVAL_SECS` and `ROLE_RESYNC_INTERVAL_SECS`
([API environment variables](/documentation/crates/api/api_server/environment_variables.md)). No feature.

## Public surface

- `spawn_all` and `WorkerHandles`: arm every worker once and keep their handles.
- One module per worker, each with its `start_*` function and its interval constants;
  `event_reservation_reevaluator::drain_due_reevaluations` and
  `runtime_session_expiry::expire_runtime_sessions` run one pass directly.
- `Error` and `Result` (a failed pass run directly, converting into `ApiError`), and `prelude`.

## Boundaries

- Depends on: `api_state`, `api_http_layer`, `api_foundation`, `api_identifiers`,
  `api_member_activity`, `api_equipment_datasets`, the domain crates `api_administration`,
  `api_identity_and_access`, `api_missions`, `api_operations` and `api_server_infrastructure`,
  sqlx, tokio, tracing, axum and thiserror.
- Used by: the API application (`crates/api/api_server`): its `api-server` binary calls `spawn_all`, and its
  integration suites run single passes.
- Rules: the API crate rules of [crates/api](/crates/api/README.md);
  `crates/api/api_server/src/tests/architecture_rules.rs` checks that only the application depends on this
  crate and that only its `api-server` binary names it.

## Related documentation

- [API background workers source](/crates/api/api_background_workers/src/README.md) — the
  workers, their intervals and the services each pass calls.
- [Fleet command ledger](/documentation/crates/api/api_server/verification_evidence/fleet_command_ledger.md)
  — the leases and expiries the fleet command reconciler enforces.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
