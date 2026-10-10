# Match telemetry services

The transactions behind the match-telemetry ingests: registering a server-scoped source match,
deciding and applying a results revision, and storing a batch of detailed events, each behind the
row lock of the registered match, with the wire-value parsers the heartbeat shares.

## Contents

```text
crates/api/api_match_telemetry/src/services/
├── ingest_parsing.rs        foreign-key errors mapped to 4xx, terrain and optional-UUID parsers
├── match_event_batches.rs   the event-batch transaction: conflict probe, idempotent insert, totals
├── match_registration.rs    the registration transaction: session ownership, insert or compare
├── match_results_ingest.rs  the results transaction: lock order, decision, writes, recomputation
├── match_revision_write.rs  the match row merge, removed-line deletes and player-line upserts
├── mod.rs                   the module tree
├── registered_match.rs      the server-scoped lookup that locks the registered match's row
├── results_revision.rs      the pure decision a revision meets against the stored match
└── tests/                   unit tests for the parsers and the decision
```

## How it works

```text
registration   session read ─► insert (server, source) or compare digest
results        lock_registered_match ─► decide_revision ─► obligated registrants ─► identities
               ─► accounts ─► match and line writes ─► attendance ─► unlinked audit
               ─► per-account statistics ─► leaderboard refresh (advisory lock, last)
event batch    lock_registered_match ─► conflict probe ─► insert ON CONFLICT DO NOTHING ─► totals
```

- **Registration.** `register_match` checks that the runtime session belongs to the caller's
  server (403 otherwise; the session may have ended), then inserts a `pending` match at revision 0
  keyed by `(server_id, source_match_id)`. A repeat of the same digest answers the stored match;
  another digest is the 409 `REGISTRATION_CONFLICT`.
- **Row lock.** `lock_registered_match` finds the match by the caller's server and the source match
  id and holds it `FOR NO KEY UPDATE`; an unregistered source is the 409 `MATCH_NOT_REGISTERED`, and
  a match recorded before registration existed (no `server_id`) is unreachable.
- **Results.** `decide_revision` classifies the revision: a strictly higher one is applied, the same
  one is inert with the same digest and the 409 `REVISION_CONFLICT` with another, a lower one is the
  409 `STALE_REVISION`, and a revision that would return a finalized match to `pending` is the 409
  `MATCH_FINALIZED`. An applied revision merges the match fields, deletes the removed lines,
  replaces the present ones (their counters only when the line carries `counters`), reconciles
  attendance through `api_member_activity::participation_attribution`, writes a
  `match.unlinked_players` system audit row for identities no account owns, recomputes each affected
  account's statistics and refreshes the leaderboard before committing. An inert retry computes the same answer read-only.
- **Event batch.** `ingest_event_batch` refuses the whole batch with the 409 `EVENT_CONFLICT` when a
  stored `event_id` has another digest, or `EVENT_SEQUENCE_CONFLICT` when a stored `sequence` has
  another `event_id`; otherwise it inserts every event idempotently and adds only the rows actually
  inserted to `match_event_totals` and `matches.event_count`. Events of a finalized match are
  accepted.

## Boundaries

- Depends on: sqlx and PostgreSQL; the domain's models and refusals; `api_server_infrastructure`
  (the runtime-session fence); `api_caller_identity` (`MachineCaller`, `lock_identities`,
  `lock_accounts`); `api_member_activity` (`participation_attribution`,
  `recompute_user_stats_on_connection`, `refresh_leaderboard_on_connection`);
  `api_audit_log` (`append_system_audit`); `api_foundation` for errors and the wire formats;
  `api_database::postgres_errors` for the Postgres error codes.
- Used by: the domain's handlers in `crates/api/api_match_telemetry/src/handlers/`; the
  heartbeat uses `ingest_parsing.rs`.
- Rules: the lock order above is fixed and shared with identity linking, which locks identities
  then accounts and never a match row, so no cycle exists; only a strictly higher revision changes
  facts; an applied revision and its derived statistics commit in one transaction; a retried or
  overlapping event batch never counts an event twice (`crates/api/api_server/tests/`
  `telemetry_revisions.rs`, `telemetry_atomicity.rs`, `detailed_events.rs`).

## Related documentation

- [Match telemetry, fleet status and derived statistics](/documentation/crates/api/api_server/design_notes/telemetry.md)
  — the revision table, the event rules and the lock order these services implement.
- [Identity transactions](/documentation/crates/api/api_server/design_notes/identity_transactions.md)
  — the identity and account lock order the results transaction shares.
