# `background_workers/`

The long-running interval tasks the API binary spawns at boot and never awaits. Each one polls or
recomputes something on a cadence so a quiet request path cannot leave shared state stale: expired
credentials, drifted event status, a cold materialized view, an unpublished server status, or a
Discord role change nobody signed in to trigger.

A worker owns the *schedule*, not the work. The query or transaction each tick performs lives in
the domain that owns the data, and the worker calls it — so the same operation is reachable from a
handler or a test without going through a timer.

## Public surface

- **`spawn_all(&AppState) -> WorkerHandles`** — arms every worker once and logs the resolved
  cadence of each. `src/bin/api.rs` calls it after migrations and before the router is built.
- **`WorkerHandles`** — one handle per worker. Holding the struct keeps the set addressable (a
  single handle can be aborted); dropping it detaches the tasks, which run until the runtime stops.
- Each worker module also exports its interval constant or resolver, so tests and the boot log read
  the same value the loop uses.

## Dependency rules

- `background_workers` is imported only by `src/bin/api.rs`. No domain and no part of `core` may
  import it; `src/tests/architecture_rules.rs` enforces that.
- Workers import `core` (for `AppState` and the pool) and the domain services that own the work.
  They contain no SQL of their own beyond the loop's own bookkeeping.

## Files

```text
mod.rs                                 Module tree, `WorkerHandles`, and `spawn_all`.
discord_role_synchronizer.rs           Scheduled Discord → web role resync.
event_lifecycle_sweeper.rs             Scheduled convergence of the stored `events.status` column.
leaderboard_refresher.rs               Scheduled refresh of the `leaderboard_totals` materialized view.
ratelimit_cleanup_worker.rs            Garbage collection for the durable rate limiter's bucket table.
server_status_publisher.rs             Scheduled republish of `server_statuses` rows onto their SSE topics.
token_purge_worker.rs                  Scheduled hard-delete of refresh-token rows long past expiry.
tests/
  discord_role_synchronizer.rs         Sibling unit tests for `discord_role_synchronizer.rs`.
  leaderboard_refresher.rs             Sibling unit tests for `leaderboard_refresher.rs`.
  server_status_publisher.rs           Sibling unit tests for `server_status_publisher.rs`.
```

Unit tests live in these sibling files, declared from the production file as
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.

## Where each tick's work lives

| Worker | Work it schedules |
|:---|:---|
| `token_purge_worker` | `identity_and_access::services::refresh_token_purge::purge_expired_refresh_tokens` |
| `discord_role_synchronizer` | `identity_and_access::services::discord_role_sync::resync_all_roles` |
| `event_lifecycle_sweeper` | `operations::services::event_lifecycle_sweep::sweep_once` |
| `leaderboard_refresher` | `command_center::services::leaderboard_view::refresh_leaderboard` |
| `server_status_publisher` | `server_infrastructure::services::status_broadcast::publish_all_server_statuses` |
| `ratelimit_cleanup_worker` | `core::middleware::durable_ratelimit::PgRateLimiter` bucket pruning |
