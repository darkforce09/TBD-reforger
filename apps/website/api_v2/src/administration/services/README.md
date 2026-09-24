# Administration services

The audit trail: the two ways a row enters `audit_logs`, the publisher that gives committed rows a
durable delivery order, and the listener and delivery stream behind the administrators' live feed.

## Contents

```text
apps/website/api_v2/src/administration/services/
├── audit_delivery.rs     the replayable stream of published audit rows, in publication order
├── audit_notifier.rs     one `LISTEN audit_log` per pool, fanned out to every open stream
├── audit_publication.rs  numbers committed audit rows in a durable publication sequence
├── audit_writer.rs       best-effort audit append, and the display name a row is attributed to
├── mod.rs                declares the modules
├── required_audit.rs     audit rows that commit inside the caller's business transaction
└── tests/                unit tests for the notifier's signals and backoff
```

## How it works

A write that must not happen without its record appends through `required_audit.rs` on its own
transaction (`append_actor_audit`, `append_system_audit` and their variants), so the action and
its audit row commit or fail together. `audit_writer::write_audit` is the best-effort path: an
audit failure is logged and never fails the action.

A trigger announces each insert with `pg_notify('audit_log', …)`. `audit_publication.rs` numbers
committed rows under a singleton lock, so a reader advancing through publication numbers can never
skip a row whose transaction committed late; the `audit_publication_worker` runs it.
`audit_delivery.rs` streams published rows from a cursor, woken by `audit_notifier.rs` and polled
on a timer as well, because a healthy notification channel does not prove a successful read. The
notifier holds one listener connection per pool, redials with backoff from 250 ms to 5 s, and tells
subscribers to resynchronise after a gap.

## Boundaries

- Depends on: the domain's models; `core` for the application state and errors; sqlx's
  `PgListener`.
- Used by:
  - the domain's audit log handlers, and `audit_publication_worker` in
    `apps/website/api_v2/src/background_workers/`;
  - `required_audit` from `identity_and_access`, `match_telemetry`, `missions`, `operations` and
    `server_infrastructure`; `audit_writer` from `command_center`, `community_content`,
    `match_telemetry`, `missions` and `server_infrastructure`;
  - the integration tests `apps/website/api_v2/tests/audit_notify.rs` and
    `apps/website/api_v2/tests/audit_publication.rs`.
- Rules: `audit_logs` is append-only; in the crate's Rust code these services are the only insert
  into it (database functions in `apps/website/api_v2/migrations/` write the others), so a domain
  that needs an audit row calls them rather than writing its own insert.
