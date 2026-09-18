# Background Workers Tests (`background_workers/tests/`)

Sibling unit test specifications for scheduled maintenance workers, ticker scheduling, advisory locking, and panic isolation.

---

## 1. Test Modules

Declared via Monorepo Law #7 (`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`).

### `token_purge_worker.rs`
- **Coverage**: Verification that expired tokens (>7 days past expiry) are deleted while retaining recently revoked tokens for reuse-detection security chains.

### `ratelimit_cleanup_worker.rs`
- **Coverage**: Pruning of inactive buckets older than 1 hour; ensuring active buckets remain unaffected.

### `lifecycle_sweeper.rs`
- **Coverage**: Simulation of advisory transaction locks (`pg_try_advisory_xact_lock(0x7BD_0225)`), preventing duplicate transitions across concurrent server instances, and correct state advancement past time horizons.

### `discord_role_synchronizer.rs`
- **Coverage**: Role differential detection, handling Discord API rate limits gracefully, and updating user role records.

### `leaderboard_refresher.rs`
- **Coverage**: Concurrent materialized view refresh execution and fallback mechanism when view is unpopulated.

### `server_status_publisher.rs`
- **Coverage**: SSE hub payload formatting, publish interval timing, and handling empty server tables without panicking.
