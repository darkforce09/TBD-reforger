# Command center services

The derived figures other domains keep current: each member's deployment count and attendance
rate, and the `leaderboard_totals` materialized view the leaderboards read.

## Contents

```text
apps/website/api_v2/src/command_center/services/
├── leaderboard_view.rs  the serialised refresh of the `leaderboard_totals` materialized view
├── mod.rs               the module tree
└── user_stats.rs        recomputes `users.total_deployments` and `users.attendance_rate` from facts
```

## Boundaries

- Depends on: sqlx; `administration` (`write_audit`, `AuditSeverity`) for the warning a failed
  best-effort recomputation records; `core` for errors.
- Used by: `match_telemetry`'s match-results ingest and `identity_and_access`'s identity linking
  (both `_on_connection` functions); `operations`' [event](/documentation_v2/glossary.md#event)
  administration and [mission](/documentation_v2/glossary.md#mission) restoration
  (`recompute_user_stats_on_connection`); `identity_and_access::services::user_lookup`
  (`ATTENDANCE_RATE_SQL`); the `leaderboard_refresher` worker in
  `apps/website/api_v2/src/background_workers/` (`refresh_leaderboard`); the integration tests in
  `apps/website/api_v2/tests/`.
- Rules: every refresh of the view takes one transaction advisory lock, and a business transaction
  takes it last, before its snapshot, so no refresher publishes an older snapshot over a newer
  commit; the two counters come from one statement in the caller's transaction, and attendance
  counts only decided observations, so a pending, waitlisted or withdrawn sign-up never becomes a
  no-show because its time has passed; no handler writes those columns itself
  (`the_sql_lives_only_in_the_service` in `apps/website/api_v2/tests/user_stats_service.rs`).
