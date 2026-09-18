# Background Workers Subsystem (`background_workers/`)

Centralized supervisory management, scheduling, and error isolation for all asynchronous background tickers and database maintenance tasks.

---

## 1. Subsystem Topology & Responsibilities

The `background_workers/` domain extracts and consolidates all 6 background tasks that were previously scattered across handlers and services:

```text
src/background_workers/
├── README.md                           <-- Domain documentation (this document)
├── mod.rs                              <-- Central supervisor: `start_all(state) -> Vec<JoinHandle<()>>` (<80 LOC)
├── token_purge_worker.rs               <-- Purges expired refresh tokens (6h interval) (<60 LOC)
├── ratelimit_cleanup_worker.rs         <-- Cleans stale rate limit buckets (1h interval) (<65 LOC)
├── lifecycle_sweeper.rs                <-- Advances event states (60s interval) (<150 LOC)
├── discord_role_synchronizer.rs        <-- Periodic guild role sync (24h interval) (<120 LOC)
├── leaderboard_refresher.rs            <-- Refreshes leaderboard MV (15m interval) (<70 LOC)
├── server_status_publisher.rs          <-- Realtime SSE status fan-out (10s interval) (<90 LOC)
│
└── tests/                              <-- Non-inline sibling unit tests
    ├── token_purge_worker.rs
    ├── lifecycle_sweeper.rs
    └── leaderboard_refresher.rs
```

---

## 2. Background Worker Schedules & Operations

| Worker Name | Former Location | Cadence / Interval | Trigger / Mechanism | SQL Operations / Purpose | Failure Behavior |
|:---|:---|:---|:---|:---|:---|
| **`token_purge_worker`** | `services/token_purge.rs:30` | Every 6 hours (`PURGE_INTERVAL`) | Immediate sweep on boot, then ticker loop. | `DELETE FROM refresh_tokens WHERE expires_at < now() - 7 days`. Retains revoked unexpired tokens for reuse detection. | Logged as error; next tick retries. |
| **`ratelimit_cleanup_worker`** | `services/ratelimit_gc.rs:40` | Every 1 hour (`RATE_LIMIT_PRUNE_INTERVAL`)| Immediate prune on boot, then ticker loop. | `DELETE FROM rate_limit_buckets WHERE updated_at < now() - 1 hour`. Reclaims stale token buckets. | Logged as warning; next tick retries. |
| **`lifecycle_sweeper`** | `handlers/events/events.rs:437`| Every 60 seconds (`LIFECYCLE_INTERVAL`) | Immediate sweep, then ticker loop. | `pg_try_advisory_xact_lock(0x7BD_0225)`. Transitions scheduled ops past `start_time` to `live`; transitions ops past end horizon to `completed`. | Logged as error; next tick retries. |
| **`discord_role_synchronizer`**| `services/role_sync.rs:151` | Every 24 hours (`ROLE_RESYNC_INTERVAL_SECS`)| Immediate check on boot, then ticker loop. | Iterates `user_discord_roles`, queries role mappings, updates `users.role` where changed. | Logged as error; next tick retries. |
| **`leaderboard_refresher`** | `db.rs:250` | Every 15 minutes (`LEADERBOARD_REFRESH_INTERVAL_SECS`)| Immediate refresh on boot, then ticker loop. | `REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_totals` (falls back to non-concurrent if empty). | Logged as error; next tick retries. |
| **`server_status_publisher`** | `realtime.rs:144` | Every 10 seconds (`SERVER_STATUS_PUBLISH_INTERVAL_SECS`)| Immediate query on boot, then ticker loop. | `SELECT ... FROM server_statuses`, serializes JSON payload, publishes to `state.hub` topic `server:{server_id}`. | Logged as error; next tick retries. |

---

## 3. Supervision & Boot Architecture

In `src/bin/api.rs`, worker startup is simplified from 6 individual ad-hoc calls into a single call:
```rust
let _workers = background_workers::start_all(&state);
```
Each worker runs in its own detached Tokio task with panic recovery. If an individual tick encounters a transient database error, it logs the failure via `tracing::error!` and sleeps until the next tick, ensuring no background worker panic can bring down the HTTP server.
