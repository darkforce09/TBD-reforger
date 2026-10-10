# API background workers source

The interval tasks the [API](/documentation/glossary/a_to_f.md#api) binary starts at boot and never
awaits. Each keeps shared state current when no request would: expired credentials, the stored
[event](/documentation/glossary/a_to_f.md#event) status, the leaderboard view, live server status,
silent [game runtimes](/documentation/glossary/g_to_m.md#game-runtime),
[fleet commands](/documentation/glossary/a_to_f.md#fleet-command) and
[mission deployments](/documentation/glossary/g_to_m.md#mission-deployment) in flight, queued
reservation re-evaluations, unpublished audit facts, Discord changes nobody signed in to pick
up, and new equipment export publications.

## Contents

```text
crates/api/api_background_workers/src/
├── audit_publication_worker.rs       publishes committed audit facts in bounded batches
├── discord_membership_reconciler.rs  re-reads members' Discord membership under Postgres leases
├── discord_role_synchronizer.rs      re-resolves every member's role from the stored Discord roles
├── equipment_export_watcher.rs       restores the equipment datasets and imports new publications
├── error.rs                          `Error` and `Result`: the failure of a pass run directly
├── event_lifecycle_sweeper.rs        converges the stored `events.status` column
├── event_reservation_reevaluator.rs  drains the queued event reservation re-evaluations
├── fleet_command_reconciler.rs       expires, re-queues or marks indeterminate the fleet commands
├── leaderboard_refresher.rs          refreshes the `leaderboard_totals` materialized view
├── lib.rs                            the crate root: the worker modules and the re-exports
├── mission_deployment_reconciler.rs  confirms or fails each server's mission deployment in flight
├── prelude.rs                        `WorkerHandles` and `spawn_all` for a glob import
├── ratelimit_cleanup_worker.rs       deletes durable rate-limit buckets idle for an hour
├── runtime_session_expiry.rs         ends silent runtime sessions and marks their servers offline
├── server_status_publisher.rs        republishes each active server's status onto its SSE topic
├── tests/                            unit tests for the three tunable intervals and their schedules
├── token_purge_worker.rs             hard-deletes refresh tokens long past expiry
└── worker_set.rs                     `WorkerHandles` and `spawn_all`, which arms every worker once
```

## How it works

`crates/api/api_server/src/bin/api_server.rs` calls `spawn_all(&state)` once, after the migrations and
before it builds the router. `spawn_all` logs the resolved intervals of the three tunable workers
and six of the fixed ones, and returns `WorkerHandles`, one Tokio task handle per worker:
aborting one stops that worker alone, and dropping them detaches the tasks, which run until the
runtime stops. Every worker makes its first pass at boot; a failed pass is logged, and the next
one tries again. The audit publication, Discord membership, reservation re-evaluation, runtime
session, fleet command and mission deployment workers also stop once the pool is closed.

A worker owns the schedule, not the work. Each pass calls a service of the domain that owns the
data, so a handler or a test reaches the same operation without a timer:

| Worker | Interval | Work it calls |
|---|---|---|
| `audit_publication_worker` | 250 ms after each pass of up to ten batches of 1000 | `api_administration::services::audit_publication::publish_audit_batch` |
| `discord_membership_reconciler` | enrolment every 30 s; a request every 40 ms, at most 32 at once | `api_identity_and_access::services::discord_rest_reconciliation`: `enroll_accounts`, `reconcile_one` |
| `discord_role_synchronizer` | `ROLE_RESYNC_INTERVAL_SECS`, 86400 by default | `api_identity_and_access::services::discord_role_sync::resync_all_roles` |
| `equipment_export_watcher` | 5 s, doubling after each failed import up to 300 s | `api_equipment_datasets::importing::generation_import`: `initialize` for both datasets once, then `poll` of the gameplay source |
| `event_lifecycle_sweeper` | 60 s | `api_operations::services::event_lifecycle_sweep::sweep_once` |
| `event_reservation_reevaluator` | 1 s | `api_member_activity::reevaluation_queue`: a leased request, then `api_operations::services::event_reservations::eligibility_reevaluation::reevaluate_event_reservations` in one transaction |
| `fleet_command_reconciler` | 5 s | `api_server_infrastructure::services::fleet_commands::command_reconciliation::reconcile_fleet_commands` |
| `leaderboard_refresher` | `LEADERBOARD_REFRESH_INTERVAL_SECS`, 900 by default | `api_member_activity::leaderboard_view::refresh_leaderboard` |
| `mission_deployment_reconciler` | 5 s | `api_missions::services::mission_deployments::deployment_settlement::reconcile_mission_deployments` |
| `ratelimit_cleanup_worker` | 1 h | `PgRateLimiter::prune` of `api_http_layer::middleware::durable_ratelimit` |
| `runtime_session_expiry` | 15 s | `api_server_infrastructure::services::runtime_sessions::expire_silent_runtime_sessions`, then a status republish per server |
| `server_status_publisher` | `SERVER_STATUS_PUBLISH_INTERVAL_SECS`, 10 by default | `api_server_infrastructure::services::status_broadcast::publish_all_server_statuses` |
| `token_purge_worker` | 6 h | `api_identity_and_access::services::refresh_token_purge::purge_expired_refresh_tokens` |

The three environment variables are read when `spawn_all` runs; a value that is not a positive
whole number of seconds means the default, and the boot log states the interval each got. The
other intervals are constants in the worker modules. The Discord membership reconciler and the
reservation re-evaluator claim their work through leases held in Postgres, so two API processes
never handle the same account or request at once.

The two passes the integration suites run directly, `drain_due_reevaluations` and
`expire_runtime_sessions`, return the crate's `Result`: its `Error` carries the `ApiError` the
domain service answered, renders that error's message in the worker's log line, and converts back
into it unchanged.

## Boundaries

- Depends on: `api_state` (`AppState`, the pool, the hub, the equipment datasets),
  `api_foundation` (`ApiError`), `api_http_layer` (`PgRateLimiter`), and the domain,
  `api_member_activity` and other API crate services in the table.
- Used by: `crates/api/api_server/src/bin/api_server.rs`, which calls `spawn_all`; integration suites under
  `crates/api/api_server/tests/` that run one pass directly (`drain_due_reevaluations`,
  `expire_runtime_sessions`) or the bucket pruning.
- Rules: only the API application's manifest depends on this crate, and in its source only
  `crates/api/api_server/src/bin/api_server.rs` names it; a worker holds no query of its own beyond
  its loop, the work stays in the owning domain's services; `spawn_all` keeps arming the bucket
  pruner (`crates/api/api_server/tests/http_infrastructure/durable_rate_limit.rs` checks `worker_set.rs` for it).

## Related documentation

- [Fleet command ledger](/documentation/crates/api/api_server/design_notes/fleet_command_ledger.md)
  — the leases and expiries the fleet command reconciler enforces.
- [Mission artifacts](/documentation/crates/api/api_server/design_notes/mission_artifacts.md)
  — how a mission deployment settles.
- [Event eligibility and allocation](/documentation/crates/api/api_server/design_notes/event_eligibility_allocation.md)
  — the re-evaluation requests the reservation worker drains.
