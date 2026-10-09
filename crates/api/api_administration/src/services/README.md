# Administration services

The audit trail's read side: the publisher that gives committed `audit_logs` rows a durable
delivery order, and the listener and delivery stream behind the administrators' live feed. The two
ways a row enters `audit_logs` live in the `api_audit_log` crate, `crates/api/api_audit_log/`.

## Contents

```text
crates/api/api_administration/src/services/
├── audit_delivery.rs     the replayable stream of `ready`, published audit rows and `reset`, in publication order
├── audit_notifier.rs     one `LISTEN audit_log` per pool, fanned out to every open stream
├── audit_publication.rs  numbers committed audit rows in a durable publication sequence
├── mod.rs                the module tree
└── tests/                unit tests for the notifier's signals and backoff, and the delivery stream's opening and resets
```

## How it works

A trigger announces each insert with `pg_notify('audit_log', …)`. `audit_publication.rs` numbers
committed rows under a singleton lock, so a reader advancing through publication numbers can never
skip a row whose transaction committed late; the `audit_publication_worker` runs it.
`audit_delivery.rs` streams published rows from a cursor, woken by `audit_notifier.rs` and polled
on a timer as well, because a healthy notification channel does not prove a successful read. A
stream yields `AuditStreamItem::Ready` first, then `AuditStreamItem::Delivery` per row. A cursor
beyond the tail or below `audit_publication_state.retained_after_sequence`, the retained floor,
yields `AuditStreamItem::Reset` and moves the stream to the tail; the check runs at open and on
every wake, and each page is read in one snapshot with the floor it is checked against. A failed
publish is logged and the read still runs; a failed read is logged, keeps the cursor, and runs
again on the next timer tick. The notifier holds one listener connection per pool, redials with
backoff from 250 ms to 5 s, and tells subscribers to resynchronise after a gap.

## Boundaries

- Depends on: the domain's models; `api_state` for the application state and `api_foundation` for errors; sqlx's
  `PgListener`.
- Used by:
  - the domain's audit log handlers, and `audit_publication_worker` in
    `crates/api/api_background_workers/src/`;
  - the integration tests `crates/api/api_server/tests/audit_notify.rs` and
    `crates/api/api_server/tests/audit_publication.rs`.
- Rules: `audit_logs` is append-only; these services never insert into it: the `api_audit_log`
  writers and the database functions in `crates/api/api_database/migrations/` do.
