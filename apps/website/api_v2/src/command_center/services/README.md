# Command center services

The derived figures other domains keep current (each member's deployment count and attendance
rate, and the `leaderboard_totals` materialized view the leaderboards read) and the dashboard's
overview of the configured game-server fleet.

## Contents

```text
apps/website/api_v2/src/command_center/services/
├── fleet_overview.rs    the active servers with their statuses, and the fleet and telemetry totals
├── leaderboard_view.rs  the serialised refresh of the `leaderboard_totals` materialized view
├── mod.rs               the module tree
├── tests/               unit tests for the fleet totals
└── user_stats.rs        recomputes `users.total_deployments` and `users.attendance_rate` from facts
```

## How it works

- **Statistics.** `recompute_user_stats_on_connection` derives one account's `total_deployments`
  and `attendance_rate` from the stored facts in one statement on the caller's connection, so the
  figures commit with the transaction that changed the facts: an applied match results revision,
  a link confirmation, unlink, relink or deleted-owner release, and event administration.
- **Leaderboard.** `refresh_leaderboard_on_connection` refreshes the `leaderboard_totals`
  materialized view inside the caller's transaction, behind a transaction advisory lock taken last;
  `refresh_leaderboard` does the same on its own connection for the scheduled worker.
- **Fleet overview.** `load_fleet_overview` reads the active servers ordered by name then id and
  their status rows through `SELECT_FLEET_STATUSES`, pairs each server with its status (none when
  it has no row), and `fleet_totals` counts the servers, the online ones and their players and
  capacity, and sums every reported telemetry queue's `backlog` and `dropped_total`.

## Boundaries

- Depends on: sqlx; `administration` (`write_audit`, `AuditSeverity`) for the warning a failed
  best-effort recomputation records; `server_infrastructure` (`ServerStatus`, `ServerStatusRow`,
  `SELECT_FLEET_STATUSES`) for the fleet's statuses; `core` for errors.
- Used by: the dashboard handler in `apps/website/api_v2/src/command_center/handlers/`
  (`load_fleet_overview`); `match_telemetry`'s match-results ingest and `identity_and_access`'s identity linking
  (both `_on_connection` functions); `operations`' [event](/documentation_v2/glossary/a_to_f.md#event)
  administration and [mission](/documentation_v2/glossary/g_to_m.md#mission) restoration
  (`recompute_user_stats_on_connection`); `identity_and_access::services::user_lookup`
  (`ATTENDANCE_RATE_SQL`); the `leaderboard_refresher` worker in
  `apps/website/api_v2/src/background_workers/` (`refresh_leaderboard`); the integration tests in
  `apps/website/api_v2/tests/`.
- Rules: every refresh of the view takes one transaction advisory lock, and a business transaction
  takes it last, before its snapshot, so no refresher publishes an older snapshot over a newer
  commit; the two counters come from one statement in the caller's transaction, and attendance
  counts only decided observations, so a pending, waitlisted or withdrawn sign-up never becomes a
  no-show because its time has passed; no handler writes those columns itself
  (`the_sql_lives_only_in_the_service` in `apps/website/api_v2/tests/user_stats_service.rs`);
  the fleet is exactly the active servers, so an inactive server never reaches the list or the
  totals (`apps/website/api_v2/tests/fleet_dashboard.rs`).
