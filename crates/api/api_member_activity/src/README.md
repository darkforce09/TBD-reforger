# API member activity source

The source of `api_member_activity`: the aggregates and queues that member-level facts feed. It
holds each member's deployment count and attendance rate, the `leaderboard_totals` materialized
view, the attendance derived from match results, and the durable queue of event eligibility
re-evaluations.

## Contents

```text
crates/api/api_member_activity/src/
├── error.rs                      `Error`: a read or write that failed, its `ApiError`, and `Result`
├── leaderboard_view.rs           the serialised refresh of the `leaderboard_totals` materialized view
├── lib.rs                        the crate root: module header, `mod` lines and the re-export of `Error`
├── participation_attribution.rs  match provenance and the attendance derived from results
├── prelude.rs                    the statistics, refresh, attribution and queue calls for glob import
├── reevaluation_queue.rs         durable re-evaluation requests, leased by the worker
└── user_stats.rs                 recomputes `users.total_deployments` and `users.attendance_rate`
```

## How it works

- **Statistics.** `recompute_user_stats_on_connection` derives one account's `total_deployments`
  and `attendance_rate` from the stored facts in one statement on the caller's connection, so the
  figures commit with the transaction that changed the facts: an applied match results revision,
  a link confirmation, unlink, relink or deleted-owner release, and event administration.
  `ATTENDANCE_RATE_SQL` is the one attendance statement, which the account lookup also reads.
- **Leaderboard.** `refresh_leaderboard_on_connection` refreshes the `leaderboard_totals`
  materialized view inside the caller's transaction, behind a transaction advisory lock taken
  last; `refresh_leaderboard` does the same on its own connection for the scheduled worker.
- **Attendance.** `participation_attribution.rs` derives attendance from finalized match results:
  a reservation active when its exact event mission's match was finalized is a no-show unless its
  player took part. The results ingest share-locks the attachments the match holds now and will
  hold (`lock_obligated_registrants`) before any identity or account lock, and
  `prior_match_accounts` names the registrants the match already records, so their accounts are
  locked too.
- **Re-evaluation queue.** Membership changes, bans and event administration only upsert a row
  in `reevaluation_queue.rs` while holding account locks; the `event_reservation_reevaluator`
  worker leases it and re-evaluates the event in its own event-first transaction.

## Boundaries

- Depends on: `api_foundation` for `ApiError`; `api_identifiers` for the account, event, match
  and mission ids; `api_audit_log` for the warning a failed best-effort recomputation records;
  sqlx.
- Used by: the API application (`crates/api/api_server`): `api_match_telemetry`'s match results ingest, `api_identity_and_access`'s identity linking,
  membership cache and account lookup, `api_operations`' event administration and mission
  restoration, `api_administration`'s bans; the `leaderboard_refresher` and
  `event_reservation_reevaluator` workers in `crates/api/api_background_workers/src/`; the integration
  tests in `crates/api/api_server/tests/`.
- Rules: every refresh of the view takes one transaction advisory lock, and a business transaction
  takes it last, before its snapshot, so no refresher publishes an older snapshot over a newer
  commit; the two counters come from one statement, and attendance counts only decided
  observations; no handler writes those columns itself (`the_sql_lives_only_in_the_service` in
  `crates/api/api_server/tests/telemetry_and_statistics/user_stats_service.rs`); producers of re-evaluation requests never take event
  locks.
