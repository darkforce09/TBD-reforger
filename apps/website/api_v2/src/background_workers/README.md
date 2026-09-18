# Background Workers Subsystem (`background_workers/`)

Every interval task the API runs, armed in one place and owned by one supervisor.

---

## 1. Subsystem Topology

```text
src/background_workers/
├── README.md                           <-- Domain documentation (this document)
├── mod.rs                              <-- Supervisor: `spawn_all(&AppState) -> WorkerHandles`
├── token_purge_worker.rs               <-- Deletes long-expired refresh tokens (6h)
├── ratelimit_cleanup_worker.rs         <-- Reclaims stale rate-limit buckets (1h)
├── event_lifecycle_sweeper.rs          <-- Converges the stored `events.status` column (60s)
├── discord_role_synchronizer.rs        <-- Re-resolves web roles from Discord snapshots (24h)
├── leaderboard_refresher.rs            <-- Refreshes the `leaderboard_totals` MV (15m)
├── server_status_publisher.rs          <-- Republishes `server_statuses` onto SSE (10s)
│
└── tests/                              <-- Non-inline sibling unit tests
    ├── discord_role_synchronizer.rs
    ├── leaderboard_refresher.rs
    └── server_status_publisher.rs
```

---

## 2. Worker Schedules & Operations

| Worker | Cadence | Mechanism | Work | Failure behaviour |
|:---|:---|:---|:---|:---|
| **`token_purge_worker`** | 6 h (`PURGE_INTERVAL`) | Boot sweep, then ticker. | `DELETE FROM refresh_tokens WHERE expires_at < now() - 7 days`. Revoked-but-unexpired rows stay — they are the reuse-detection tripwire. | Logged as error; next tick retries. |
| **`ratelimit_cleanup_worker`** | 1 h (`RATE_LIMIT_PRUNE_INTERVAL`) | Boot prune, then ticker. | Deletes `rate_limit_buckets` rows untouched for `RATE_LIMIT_BUCKET_TTL`. A bucket that idle has already refilled, so this can never grant quota. | Logged as warning; next tick retries. |
| **`event_lifecycle_sweeper`** | 60 s (`LIFECYCLE_INTERVAL`) | Ticker calling `handlers::events::sweep_once`. | Under `pg_try_advisory_xact_lock`, moves started operations to `live` and operations past their end horizon to `completed`, auditing both. | Logged as error; next tick retries. |
| **`discord_role_synchronizer`** | 24 h (`ROLE_RESYNC_INTERVAL_SECS`) | Boot pass, then ticker. | `identity_and_access::services::discord_role_sync::resync_all_roles` — re-resolves each user's web tier from stored `user_discord_roles` against current mappings. | Logged as error; next tick retries. |
| **`leaderboard_refresher`** | 15 m (`LEADERBOARD_REFRESH_INTERVAL_SECS`) | Boot refresh, then ticker. | `REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_totals` (non-concurrent when unpopulated). | Logged as error; next tick retries. |
| **`server_status_publisher`** | 10 s (`SERVER_STATUS_PUBLISH_INTERVAL_SECS`) | Boot poll, then ticker. | Reads `server_statuses` and publishes each row to the hub topic `server:{server_id}`. | Logged as error; next tick retries. |

Every worker is a safety net for a request path that already does the same work in-request
(ingest, OAuth login, admin sync). None of them is load-bearing: a late, skipped, or doubled
tick changes no user-visible decision.

---

## 3. Supervision & Boot

`src/bin/api.rs` arms the whole set with one call:

```rust
let _workers = background_workers::spawn_all(&state);
```

`spawn_all` resolves the three env-tunable cadences, logs what each worker actually got, and
returns [`WorkerHandles`] — one field per worker, so a caller can abort one without touching
the others. Each worker runs in its own detached Tokio task and swallows tick failures into
`tracing`, so no background failure can take the HTTP server down.
