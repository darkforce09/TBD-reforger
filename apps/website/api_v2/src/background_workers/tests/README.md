# Background Workers Tests (`background_workers/tests/`)

Sibling unit tests for the worker modules, declared via Monorepo Law #7
(`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`).

---

## Test Modules

### `discord_role_synchronizer.rs`
- **Coverage**: `ROLE_RESYNC_INTERVAL_SECS` parsing (default, positive, zero / negative / garbage) and a stubbed scheduler proving the resync runs on boot and again on each interval tick.

### `leaderboard_refresher.rs`
- **Coverage**: `LEADERBOARD_REFRESH_INTERVAL_SECS` parsing and a stubbed refresh proving the boot-plus-interval schedule.

### `server_status_publisher.rs`
- **Coverage**: `SERVER_STATUS_PUBLISH_INTERVAL_SECS` parsing and a stubbed publish tick proving the boot-plus-interval schedule.

The remaining three workers hold no sibling tests: `token_purge_worker`,
`ratelimit_cleanup_worker`, and `event_lifecycle_sweeper` are thin tickers over query
functions covered by the integration suites (`tests/lifecycle_extra.rs`,
`tests/t578_ratelimit.rs`).
