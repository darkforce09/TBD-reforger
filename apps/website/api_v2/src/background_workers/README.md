# Background workers

The interval tasks the [API](/documentation_v2/glossary.md#api) binary starts at boot and never
awaits. Each keeps shared state current when no request would: expired credentials, the stored
[event](/documentation_v2/glossary.md#event) status, the leaderboard view, live server status,
silent [game runtimes](/documentation_v2/glossary.md#game-runtime),
[fleet commands](/documentation_v2/glossary.md#fleet-command) and
[mission deployments](/documentation_v2/glossary.md#mission-deployment) in flight, queued
reservation re-evaluations, unpublished audit facts, and Discord changes nobody signed in to pick
up.

## Contents

```text
apps/website/api_v2/src/background_workers/
├── audit_publication_worker.rs       publishes committed audit facts in bounded batches
├── discord_membership_reconciler.rs  re-reads members' Discord membership under Postgres leases
├── discord_role_synchronizer.rs      re-resolves every member's role from the stored Discord roles
├── event_lifecycle_sweeper.rs        converges the stored `events.status` column
├── event_reservation_reevaluator.rs  drains the queued event reservation re-evaluations
├── fleet_command_reconciler.rs       expires, re-queues or marks indeterminate the fleet commands
├── leaderboard_refresher.rs          refreshes the `leaderboard_totals` materialized view
├── mission_deployment_reconciler.rs  confirms or fails each server's mission deployment in flight
├── mod.rs                            `WorkerHandles` and `spawn_all`, which arms every worker once
├── ratelimit_cleanup_worker.rs       deletes durable rate-limit buckets idle for an hour
├── runtime_session_expiry.rs         ends silent runtime sessions and marks their servers offline
├── server_status_publisher.rs        republishes every server's status onto its SSE topic
├── tests/                            unit tests for the three tunable intervals and their schedules
└── token_purge_worker.rs             hard-deletes refresh tokens long past expiry
```

## How it works

`apps/website/api_v2/src/bin/api.rs` calls `spawn_all(&state)` once, after the migrations and
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
| `audit_publication_worker` | 250 ms after each pass of up to ten batches of 1000 | `administration::services::audit_publication::publish_audit_batch` |
| `discord_membership_reconciler` | enrolment every 30 s; a request every 40 ms, at most 32 at once | `identity_and_access::services::discord_rest_reconciliation`: `enroll_accounts`, `reconcile_one` |
| `discord_role_synchronizer` | `ROLE_RESYNC_INTERVAL_SECS`, 86400 by default | `identity_and_access::services::discord_role_sync::resync_all_roles` |
| `event_lifecycle_sweeper` | 60 s | `operations::services::event_lifecycle_sweep::sweep_once` |
| `event_reservation_reevaluator` | 1 s | `operations::services::event_reservations`: a leased request from `reevaluation_queue`, then `eligibility_reevaluation::reevaluate_event_reservations` in one transaction |
| `fleet_command_reconciler` | 5 s | `server_infrastructure::services::fleet_commands::command_reconciliation::reconcile_fleet_commands` |
| `leaderboard_refresher` | `LEADERBOARD_REFRESH_INTERVAL_SECS`, 900 by default | `command_center::services::leaderboard_view::refresh_leaderboard` |
| `mission_deployment_reconciler` | 5 s | `missions::services::mission_deployments::deployment_settlement::reconcile_mission_deployments` |
| `ratelimit_cleanup_worker` | 1 h | `PgRateLimiter::prune` of `core::middleware::durable_ratelimit` |
| `runtime_session_expiry` | 15 s | `server_infrastructure::services::runtime_sessions::expire_silent_runtime_sessions`, then a status republish per server |
| `server_status_publisher` | `SERVER_STATUS_PUBLISH_INTERVAL_SECS`, 10 by default | `server_infrastructure::services::status_broadcast::publish_all_server_statuses` |
| `token_purge_worker` | 6 h | `identity_and_access::services::refresh_token_purge::purge_expired_refresh_tokens` |

The three environment variables are read when `spawn_all` runs; a value that is not a positive
whole number of seconds means the default, and the boot log states the interval each got. The
other intervals are constants in the worker modules. The Discord membership reconciler and the
reservation re-evaluator claim their work through leases held in Postgres, so two API processes
never handle the same account or request at once.

## Boundaries

- Depends on: `crate::core` (`AppState`, the pool, the hub, `ApiError`, `PgRateLimiter`) and the
  domain services in the table.
- Used by: `apps/website/api_v2/src/bin/api.rs`, which calls `spawn_all`; integration suites under
  `apps/website/api_v2/tests/` that run one pass directly (`drain_due_reevaluations`,
  `expire_runtime_sessions`) or the bucket pruning.
- Rules: nothing in `apps/website/api_v2/src/` imports this module except `bin/api.rs` and
  `lib.rs` (`background_workers_used_only_by_the_binary` in
  `apps/website/api_v2/src/tests/architecture_rules.rs`); a worker holds no query of its own beyond
  its loop, the work stays in the owning domain's services; `spawn_all` keeps arming the bucket
  pruner (`apps/website/api_v2/tests/durable_rate_limit.rs` checks `mod.rs` for it).

## Related documentation

- [Fleet command ledger](/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md)
  — the leases and expiries the fleet command reconciler enforces.
- [Mission artifacts](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md)
  — how a mission deployment settles.
- [Event eligibility and allocation](/documentation_v2/website/api_v2/verification_evidence/event_eligibility_allocation.md)
  — the re-evaluation requests the reservation worker drains.
