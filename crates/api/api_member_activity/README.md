# API member activity

The `api_member_activity` crate: the aggregates and queues that member-level facts of the
[API](/documentation/glossary/a_to_f.md#api) feed. It keeps each member's deployment count and
attendance rate, the `leaderboard_totals` materialized view, the attendance derived from match
results and the durable queue of event eligibility re-evaluations current, so the domains that
change those facts update them without importing each other.

## Contents

```text
crates/api/api_member_activity/
├── Cargo.toml  the package: `api_audit_log`, `api_foundation`, `api_identifiers`, sqlx (`postgres`), layout tier 3
└── src/        the statistics, the leaderboard refresh, the attendance attribution, the re-evaluation queue, the error and the prelude
```

## How it works

Every writer runs on its caller's business transaction. A match results revision, a link
confirmation, an unlink or a ban recomputes the affected members' counters in one statement and
refreshes the leaderboard view last, behind a transaction advisory lock, so the figures commit
with the change that moved them and no refresher publishes an older snapshot over a newer commit.
Membership changes and event administration only upsert a re-evaluation request while holding
their account locks; the API's `event_reservation_reevaluator` worker leases the request and
re-evaluates the event in its own transaction. The maintenance wrappers
(`recompute_user_stats_best_effort`, `refresh_leaderboard_best_effort`) record a warning audit
line instead of failing their caller.

## Getting started

Run from the repository root:

```bash
cargo clippy -p api_member_activity --all-targets -- -D warnings
cargo xtask db test-it --test user_stats_service --test eligibility_release_transactions
```

The crate has no unit tests of its own: every writer is SQL against the `users`,
`event_registrations`, `leaderboard_totals` and `event_reservation_reevaluations` relations,
which the API's integration suites prove against Postgres.

## Configuration

No feature and no variable; the caller passes the pool or the connection.

## Public surface

- `user_stats`: `recompute_user_stats`, `recompute_user_stats_on_connection`,
  `recompute_user_stats_best_effort`, `refresh_leaderboard_best_effort` and
  `ATTENDANCE_RATE_SQL`.
- `leaderboard_view`: `refresh_leaderboard` and `refresh_leaderboard_on_connection`.
- `participation_attribution`: `prior_match_accounts`, `reconcile_match`, `refresh_attendance`
  and `lock_obligated_registrants`.
- `reevaluation_queue`: `request_reevaluation`, `request_reevaluation_for_account`,
  `schedule_pool_openings`, `claim_due_reevaluation`, `complete_reevaluation`,
  `fail_reevaluation` and `ReevaluationLease`.
- `Error` and `Result` (a read or write failure, converting into `ApiError`), and `prelude`.

## Boundaries

- Depends on: `api_audit_log`, `api_foundation`, `api_identifiers`, sqlx, chrono, thiserror and
  uuid; the tables and the view of `crates/api/api_database/migrations/`.
- Used by: the API application (`apps/api`): the identity and access, match telemetry,
  operations and administration domains, the background workers and the integration suites.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); nothing here names a domain
  or `api_caller_identity`.

## Related documentation

- [API member activity source](/crates/api/api_member_activity/src/README.md) — the files and how
  statistics, the leaderboard, attendance and the queue work.
- [API audit log](/crates/api/api_audit_log/README.md) — the warning lines the maintenance
  wrappers record.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
