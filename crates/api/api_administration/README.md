# API administration

The `api_administration` crate: the [API](/documentation/glossary/a_to_f.md#api)'s
[administration](/documentation/glossary/a_to_f.md#administration) domain. It holds the member
roster and the moderation taken against it (bans, ban lifts, warnings), the Discord
[role](/documentation/glossary/n_to_z.md#role) resync, the membership grace extension, and the
audit log that records every privileged action, with its publication sequence, its CSV export and
its live feed, together with the `/api/v1/admin/*` route table the API's router merges.

## Contents

```text
crates/api/api_administration/
├── Cargo.toml  the package: `api_state`, `api_identity_and_access`, `api_caller_identity`, `api_member_activity`, `api_audit_log`, `api_http_layer`, sqlx (`postgres`), axum, layout tier 7
└── src/        the route table, the handlers, the audit notifier, publication and delivery services, the models, the error and the prelude
```

## How it works

Every route sits under `/api/v1/admin/*` and takes `AdminUser`, apart from the membership grace
extension, which takes `AuthUser` and lets `api_identity_and_access` require verified administrator
authority, so it still works while Discord is unreachable. A member's role follows their Discord
roles: the role edit route refuses every change, and the resync route re-applies the mapping.

Privileged writes across the API leave a row in `audit_logs` through `api_audit_log`. The
publication pass numbers committed rows in a durable sequence; the live feed listens for
`pg_notify('audit_log', …)`, delivers the published rows in sequence order, replays from a client's
`Last-Event-ID`, and answers a cursor it cannot replay with `event: reset` and the tail. The source
tree README has the detail.

## Getting started

Run from the repository root:

```bash
cargo test -p api_administration
cargo clippy -p api_administration --all-targets -- -D warnings
cargo xtask db test-it --test audit_publication --test audit_frontend --test audit_notify --test route_acceptance_administration_center_content
```

The unit tests cover the CSV export's formula escaping, the stream events, the roster paging, the
notifier's signals and backoff, and the delivery stream's opening and resets; the SQL, the
publication sequence and the live feed are proved against Postgres by the API's integration
suites.

## Configuration

No feature and no variable of its own.

## Public surface

- `routes()`: the domain's `/api/v1` route table.
- `handlers`: the roster, discipline, role, grace and audit log handlers, each with its
  `/// @route` tag; `handlers::audit_logs::audit_row_stream`, the live feed's rows alone.
- `services`: `audit_publication::publish_audit_batch` (the publication pass),
  `audit_delivery::audit_delivery_stream` (the stream of `AuditStreamItem`s behind the live feed)
  and `audit_notifier::AuditNotify` (the per-pool `LISTEN audit_log` fan-out).
- `models`: `AuditLog`, `AuditStreamReady`, `AuditStreamReset`, `PersonnelPage`, `PersonnelRow`
  and `Warning`.
- `Error` and `Result` (a read or write failure, converting into `ApiError`), and `prelude`.

## Boundaries

- Depends on: `api_state`, `api_identity_and_access` (the role resync and the grace extension),
  `api_caller_identity`, `api_member_activity`, `api_audit_log`, `api_http_layer`,
  `api_foundation`, `api_identifiers`, `fleet_wire_contract`, sqlx, axum, serde,
  chrono, csv, tokio, futures, async-stream, tracing and thiserror. It names no other domain.
- Used by: the API application (`crates/api/api_server`): its router merges `routes`, its audit publication
  worker runs `publish_audit_batch`, and its audit integration suites drive the services directly;
  over HTTP, the personnel and audit logs pages of the single-page app.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); its route table, its handlers
  and its imports follow the domain graph.

## Related documentation

- [API administration source](/crates/api/api_administration/src/README.md) — the files, the
  routes and how the roster, the moderation and the audit stream work.
- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes.
- [Administration and community content](/documentation/crates/api/api_server/verification_evidence/administration_and_content.md)
  — the roster paging, the audit stream's replay, reset and recovery semantics.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
